;;; trace_capture.lisp — per-fn (args, result) capture via sb-int:encapsulate.
;;;
;;; Loaded by extractor_worker.lisp. Defines the :ICHI-TRACE package.
;;; Wraps named functions with a recorder that:
;;;   1. forwards to the original definition,
;;;   2. captures (args, result) as readably-printed Lisp strings,
;;;   3. discards calls whose args or result aren't primitive-printable
;;;      (closures, instances, hash-tables, opaque structs).
;;;
;;; The worker pulls captures via DRAIN between requests; the FastAPI
;;; driver ships them to a Python writer that partitions by FQN and
;;; writes per-symbol parquet files.
;;;
;;; All capture state is process-global (no thread-locals). The pool
;;; serializes one request at a time per worker, so no contention.

(defpackage :ichi-trace
  (:use :cl)
  (:export #:install
           #:install-many
           #:uninstall
           #:uninstall-all
           #:installed
           #:captures
           #:n-captures
           #:clear
           #:drain
           #:n-skipped
           #:*in-recorder*))

(in-package :ichi-trace)

;; Re-entrance guard. Bound true ONLY while the recorder is doing its
;; own bookkeeping (printing args/result, pushing to *captures*). It is
;; deliberately NOT bound around the call to BASIC-DEF, so inner calls
;; to other installed fns ARE captured — that's the whole point of
;; installing internal callees alongside entry points. The guard exists
;; purely to keep safe-prin1 (or anything it triggers) from re-entering
;; the recorder and corrupting *captures*.
(defparameter *in-recorder* nil)

;; FQN string -> (sym . result-projector-or-nil) for every fn currently
;; encapsulated. The projector is a function called on RESULTS (the
;; multiple-value-list of the original return) before SAFE-PRIN1, so
;; non-readable shapes (DAO instances, opaque structs) can be flattened
;; into primitive-printable plists per-FQN before capture. NIL means
;; the raw RESULTS goes to SAFE-PRIN1 unchanged (the original behavior).
(defparameter *installed* (make-hash-table :test 'equal))

;; List of (fqn args-string result-string), most-recent first. DRAIN
;; reverses to chronological order on the way out.
(defparameter *captures* nil)

;; Skip counter: incremented when args or result fail the primitive
;; gate. Diagnostic; cleared by CLEAR/DRAIN.
(defparameter *skipped* 0)


;; --- printability gate -----------------------------------------------------

(defun primitive-p (v)
  "True if V's printed representation can round-trip via READ. Filters
   out values whose prin1 produces #<...> syntax that LEXPR can't
   parse on the Rust replay side."
  (typecase v
    ((or null number character string keyword) t)
    (symbol (and (symbol-package v) t))
    (cons (and (primitive-p (car v))
               (primitive-p (cdr v))))
    ((simple-array t (*)) (every #'primitive-p (coerce v 'list)))
    (vector (every #'primitive-p (coerce v 'list)))
    (t nil)))

(defun tag-chars (v)
  "Recursively replace every CHARACTER inside V with the tagged list
   (:CHAR <codepoint>). The Rust replay parser deliberately doesn't
   ship a Unicode-name table — without this tagging, real captures
   contain forms like #\\HIRAGANA_LETTER_HA that no S-expr reader
   on the Rust side resolves. (:CHAR n) is just a 2-element list,
   readable by anything."
  (typecase v
    (character (list :char (char-code v)))
    (cons      (cons (tag-chars (car v)) (tag-chars (cdr v))))
    (vector    (if (stringp v) v (map 'vector #'tag-chars v)))
    (t         v)))

(defun safe-prin1 (v)
  "Return V's prin1 string, or NIL if V is not primitive-printable.
   *PRINT-READABLY* is bound true so strings keep their quotes and
   escapes in a form the Rust S-expr parser can parse back. Characters
   are pre-walked into (:CHAR codepoint) form to dodge Unicode-name
   syntax SBCL would otherwise emit."
  (when (primitive-p v)
    (handler-case
        (let ((tagged (tag-chars v))
              (*print-readably* t)
              (*print-circle* t)
              (*print-pretty* nil)
              (*print-length* nil)
              (*print-level* nil))
          (with-output-to-string (s) (prin1 tagged s)))
      (error () nil))))


;; --- recorder factory ------------------------------------------------------

;; SBCL's sb-int:encapsulate calling convention:
;;
;;   (sb-int:encapsulate NAME TAG WRAPPER)
;;
;; replaces (fdefinition NAME) with a function that, when called with
;; the user's arguments, invokes WRAPPER as:
;;
;;   (funcall WRAPPER ORIGINAL-FN ARG1 ARG2 ... ARGN)
;;
;; The user's args are SPREAD (not packed in a list). ORIGINAL-FN is
;; the prior fdefinition. Returning multiple values is the wrapper's
;; responsibility — verified via toy-fn smoke test on SBCL 2.2.9.

(defparameter *projection-failed* '#:projection-failed
  "Sentinel returned by APPLY-PROJECTOR when a projector signals an error.
   Distinct from any user value (uninterned), so NIL projector outputs
   don't collide with the failure path.")

(defun apply-projector (projector value)
  "Run PROJECTOR on VALUE inside HANDLER-CASE; return *PROJECTION-FAILED*
   on error so the caller can count the call as skipped without aborting
   the trace. When PROJECTOR is nil, VALUE passes through unchanged."
  (if projector
      (handler-case (funcall projector value)
        (error () *projection-failed*))
      value))

(defun default-arg-projector ()
  (let ((sym (find-symbol "FLATTEN-ARGS" :ichi-projectors)))
    (and sym (fboundp sym) (symbol-function sym))))

(defun default-result-projector ()
  (let ((sym (find-symbol "FLATTEN-RESULTS" :ichi-projectors)))
    (and sym (fboundp sym) (symbol-function sym))))

(defun make-recorder (fqn arg-projector result-projector encoder)
  ;; Resolve defaults at recorder-build time. NIL means "no projection";
  ;; install passes T to mean "use the package default flatten".
  ;;
  ;; ENCODER selects the serializer applied to the projector output:
  ;;   :sexp (default) — SAFE-PRIN1, producing Lisp source text. The
  ;;     Rust audit side parses via the hand-rolled kani::sexp reader.
  ;;     This is the legacy path; all FQNs use it unless overridden.
  ;;   :json — TO-JSON-STRING from :ichi-projectors-json. Produces JSON
  ;;     suitable for serde_json on the Rust audit side. Projectors must
  ;;     return jsown-compatible trees (lists for arrays, (:obj (k . v)
  ;;     ...) for objects), not the raw tagged-plist form that the
  ;;     legacy s-expr projector produces. Pair with the JSON projector
  ;;     package's FLATTEN-ARGS-JSON / FLATTEN-RESULTS-JSON.
  ;;
  ;; Re-entrance guard scope: *in-recorder* is bound only around the
  ;; projector and serializer calls. Their internals can themselves call
  ;; into installed fns (closer-mop slot walks, prin1 methods, etc.) and
  ;; we don't want those to recurse into capture. The push to *captures*
  ;; runs unconditionally — so an installed FQN called inside another
  ;; installed FQN's body still records its own capture, matching the
  ;; documented "inner installed callees ARE captured" contract.
  (let ((arg-fn    (cond ((eq arg-projector    t) (default-arg-projector))
                         (t arg-projector)))
        (result-fn (cond ((eq result-projector t) (default-result-projector))
                         (t result-projector)))
        (serialize (ecase encoder
                     (:sexp (lambda (v) (safe-prin1 v)))
                     (:json (let ((sym (find-symbol "TO-JSON-STRING"
                                                    :ichi-projectors-json)))
                              (unless (and sym (fboundp sym))
                                (error "ICHI-PROJECTORS-JSON:TO-JSON-STRING not available — \
                                        load projectors_json.lisp before installing with :encoder :json"))
                              (let ((encoder-fn (symbol-function sym)))
                                (lambda (v)
                                  (handler-case (funcall encoder-fn v)
                                    (error () nil)))))))))
    (lambda (basic-def &rest args)
      (let ((results (multiple-value-list (apply basic-def args))))
        (let* ((projected-args    (let ((*in-recorder* t))
                                    (apply-projector arg-fn args)))
               (projected-results (let ((*in-recorder* t))
                                    (apply-projector result-fn results)))
               (args-str   (and (not (eq projected-args    *projection-failed*))
                                (not (eq projected-results *projection-failed*))
                                (let ((*in-recorder* t))
                                  (funcall serialize projected-args))))
               (result-str (and args-str
                                (let ((*in-recorder* t))
                                  (funcall serialize projected-results)))))
          (if (and args-str result-str)
              (push (list fqn args-str result-str) *captures*)
              (incf *skipped*)))
        (values-list results)))))


;; --- FQN parsing -----------------------------------------------------------

(defun resolve-symbol (fqn-string)
  "Parse 'PKG:SYMBOL' or 'PKG::SYMBOL' into a function symbol.
   Signals an error if the package is missing, the symbol is not
   interned in it, or the symbol is not fbound."
  (let* ((sep (or (position #\: fqn-string)
                  (error "no package separator in ~a" fqn-string)))
         (pkg-name (subseq fqn-string 0 sep))
         (sym-start (if (and (> (length fqn-string) (1+ sep))
                             (char= (aref fqn-string (1+ sep)) #\:))
                        (+ sep 2)
                        (1+ sep)))
         (sym-name (subseq fqn-string sym-start))
         (pkg (or (find-package pkg-name)
                  (error "no package: ~a" pkg-name)))
         (sym (or (find-symbol sym-name pkg)
                  (error "no symbol ~a in ~a" sym-name pkg-name))))
    (unless (fboundp sym)
      (error "symbol ~a is not fbound" fqn-string))
    sym))


;; --- public API ------------------------------------------------------------

(defun install (fqn-string &key (arg-projector t) (result-projector t) (encoder :sexp))
  "Install a recorder for FQN-STRING. ARG-PROJECTOR / RESULT-PROJECTOR
   each accept three values: T (default) uses the package-level
   ICHI-PROJECTORS:FLATTEN-ARGS / FLATTEN-RESULTS generic flatten;
   a function value is called as a custom projector (it receives the
   raw args list / multiple-value-list of the return); NIL disables
   projection on that side and the raw value goes straight to the
   selected ENCODER.

   ENCODER controls the serializer:
     :SEXP (default) — SAFE-PRIN1 of the projector output (legacy).
     :JSON           — ICHI-PROJECTORS-JSON:TO-JSON-STRING of the
                       projector output. Projectors must return
                       jsown-compatible trees; usually paired with
                       :ARG-PROJECTOR ICHI-PROJECTORS-JSON:FLATTEN-ARGS-JSON
                       and :RESULT-PROJECTOR FLATTEN-RESULTS-JSON.

   Re-installing swaps the wrapper — the call is NOT idempotent in the
   projector / encoder arguments."
  (let ((sym (resolve-symbol fqn-string)))
    (when (gethash fqn-string *installed*)
      (sb-int:unencapsulate sym 'ichi-trace))
    (sb-int:encapsulate sym 'ichi-trace
                        (make-recorder fqn-string arg-projector result-projector encoder))
    (setf (gethash fqn-string *installed*)
          (list sym arg-projector result-projector encoder))
    fqn-string))

(defun install-many (specs)
  "Install each SPEC. SPEC is either a string FQN (no projector) or a
   list (FQN-STRING &key result-projector ...) suitable for APPLY."
  (mapcar (lambda (spec)
            (etypecase spec
              (string (install spec))
              (cons   (apply #'install spec))))
          specs))

(defun uninstall (fqn-string)
  (let ((entry (gethash fqn-string *installed*)))
    (when entry
      (sb-int:unencapsulate (car entry) 'ichi-trace)
      (remhash fqn-string *installed*))
    fqn-string))

(defun uninstall-all ()
  (loop for entry being the hash-values of *installed*
        do (sb-int:unencapsulate (car entry) 'ichi-trace))
  (clrhash *installed*))

(defun installed ()
  (loop for fqn being the hash-keys of *installed* collect fqn))

(defun n-captures () (length *captures*))
(defun n-skipped () *skipped*)

(defun captures ()
  "Return all captures in chronological order. Non-destructive."
  (reverse *captures*))

(defun clear ()
  "Discard all buffered captures and reset the skip counter."
  (setf *captures* nil
        *skipped* 0))

(defun drain ()
  "Return all captures (chronological order) and clear the buffer
   atomically. The single-worker-per-request invariant of the pool
   makes the read-then-clear sequence safe."
  (let ((result (reverse *captures*)))
    (setf *captures* nil
          *skipped* 0)
    result))
