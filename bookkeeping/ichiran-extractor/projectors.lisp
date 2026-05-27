;;; projectors.lisp — generic value flattener for the trace recorder.
;;;
;;; Loaded by extractor_worker.lisp after trace_capture.lisp. Defines
;;; the :ICHI-PROJECTORS package, whose centerpiece is the generic
;;; function FLATTEN-TRACE-VALUE: walk an arbitrary value and return a
;;; primitive-printable equivalent that SAFE-PRIN1 can serialize.
;;;
;;; The recorder calls FLATTEN-TRACE-VALUE on every args / results
;;; tuple by default. POSTMODERN dao-class instances and SBCL
;;; structures get expanded into (:CLASS NAME :slot val ...) plists
;;; uniformly, so adding new DAOs to the trace plan needs no
;;; per-FQN projector glue. For DAOs we emit column slots first
;;; (preserving the legacy layout) followed by non-column CLOS slots
;;; — necessary because functions like ICHIRAN/DICT::BEST-KANA-CONJ
;;; branch on session-only slots (WORD-CONJUGATIONS, HINTEDP) inherited
;;; from SIMPLE-TEXT, so an args projection that omits them collapses
;;; semantically distinct inputs to identical printed args and triggers
;;; spurious nondeterminism warnings in the writer. *OMIT-SLOTS*
;;; suppresses individual slots — used today to drop ENTRY.CONTENT,
;;; the JMdict XML blob that's only consumed by admin tooling.
;;;
;;; If a port needs non-default behavior (synthesized slot, dropped
;;; field, alternate shape) it can still install with an explicit
;;; :ARG-PROJECTOR / :RESULT-PROJECTOR override that bypasses the
;;; default flatten on that side.

(defpackage :ichi-projectors
  (:use :cl)
  (:export #:flatten-trace-value
           #:flatten-args
           #:flatten-results
           #:*omit-slots*))

(in-package :ichi-projectors)

(defparameter *omit-slots*
  '((ichiran/dict::entry ichiran/dict::content))
  "Per-class slot blocklist for FLATTEN-TRACE-VALUE. Each entry is
   (CLASS-NAME SLOT-NAME ...). The default flatten skips listed slots
   when expanding (:CLASS X :slot ...) plists. Used to drop the JMdict
   XML blob on ENTRY (max 5.4 KB, total 67 MB across the live DB);
   its only readers are admin paths — diff-entries in
   ichiran/maintenance and load-sense-props in dict-fix.lisp.")

(defun %class-keyword (instance)
  (intern (symbol-name (class-name (class-of instance))) :keyword))

(defun %slot-keyword (slot-name)
  (intern (symbol-name slot-name) :keyword))

(defun %slot-omitted-p (class-name slot-name)
  (loop for entry in *omit-slots*
        thereis (and (eq (first entry) class-name)
                     (member slot-name (rest entry)))))

(defun %safe-slot (instance slot-name)
  "Read SLOT-NAME from INSTANCE without ever signaling. Unbound slots
   and lazy DAO slots that error on access become NIL — projection
   never aborts a trace."
  (handler-case
      (and (slot-boundp instance slot-name)
           (slot-value instance slot-name))
    (error () nil)))

(defgeneric flatten-trace-value (v)
  (:documentation
   "Return a primitive-printable equivalent of V suitable for
    SAFE-PRIN1. Numbers, strings, booleans, keywords, packaged
    symbols, and characters ride through. CONS and VECTOR recurse.
    SBCL structures and POSTMODERN dao-class instances expand into
    (:CLASS NAME :slot val ...) plists, omitting any slot listed in
    *OMIT-SLOTS*. Other instances (closures, hash-tables, plain CLOS
    objects) pass through untouched and let SAFE-PRIN1 skip the call."))

(defmethod flatten-trace-value (v) v)

(defmethod flatten-trace-value ((v cons))
  (cons (flatten-trace-value (car v))
        (flatten-trace-value (cdr v))))

(defmethod flatten-trace-value ((v vector))
  (if (stringp v) v (map 'vector #'flatten-trace-value v)))

(defmethod flatten-trace-value ((v structure-object))
  (let* ((cls (class-of v))
         (cls-name (class-name cls)))
    (list* :class
           (%class-keyword v)
           (loop for s in (closer-mop:class-slots cls)
                 for sn = (closer-mop:slot-definition-name s)
                 unless (%slot-omitted-p cls-name sn)
                 collect (%slot-keyword sn)
                 and collect (flatten-trace-value (%safe-slot v sn))))))

(defmethod flatten-trace-value ((v standard-object))
  ;; DAO rows are STANDARD-OBJECTs whose CLASS-OF is a POSTMODERN:DAO-CLASS.
  ;; POSTMODERN::DAO-COLUMN-SLOTS lists the column-mapped slots in
  ;; declaration order; CLOSER-MOP:CLASS-SLOTS lists every slot on the
  ;; class. We emit column slots first (preserving the legacy printed
  ;; layout that earlier captures used) followed by any inherited or
  ;; non-column CLOS slots. This is required because session-only slots
  ;; like SIMPLE-TEXT.CONJUGATIONS / HINTEDP are read by functions such
  ;; as BEST-KANA-CONJ and BEST-KANJI-CONJ — omitting them lets two
  ;; behaviorally-distinct inputs project to the same args string and
  ;; surface as spurious nondeterminism in the writer.
  ;;
  ;; Non-DAO standard-objects (e.g. ICHIRAN/DICT::COUNTER-TEXT and its
  ;; subclasses) get the same closer-mop slot walk — without it the
  ;; class-of falls through to the identity method, the value fails the
  ;; PRIMITIVE-P gate at SAFE-PRIN1 time, and the recorder records a
  ;; skip rather than a capture. Closures and built-in non-class
  ;; standard-objects (FUNCTION, etc.) don't have CLOS slots; they fall
  ;; through to the next method.
  (let ((cls (class-of v)))
    (cond
      ((typep cls 'postmodern:dao-class)
       (let* ((cls-name (class-name cls))
              (column-slots (postmodern::dao-column-slots cls))
              (column-names (mapcar #'closer-mop:slot-definition-name
                                    column-slots))
              (extra-slots
                (remove-if (lambda (s)
                             (member (closer-mop:slot-definition-name s)
                                     column-names))
                           (closer-mop:class-slots cls))))
         (list* :class
                (%class-keyword v)
                (nconc
                 (loop for s in column-slots
                       for sn = (closer-mop:slot-definition-name s)
                       unless (%slot-omitted-p cls-name sn)
                       collect (%slot-keyword sn)
                       and collect (flatten-trace-value (%safe-slot v sn)))
                 (loop for s in extra-slots
                       for sn = (closer-mop:slot-definition-name s)
                       unless (%slot-omitted-p cls-name sn)
                       collect (%slot-keyword sn)
                       and collect (flatten-trace-value (%safe-slot v sn)))))))
      ((typep cls 'standard-class)
       (let ((cls-name (class-name cls)))
         (list* :class
                (%class-keyword v)
                (loop for s in (closer-mop:class-slots cls)
                      for sn = (closer-mop:slot-definition-name s)
                      unless (%slot-omitted-p cls-name sn)
                      collect (%slot-keyword sn)
                      and collect (flatten-trace-value (%safe-slot v sn))))))
      (t (call-next-method)))))

(defun flatten-args (args)
  "Walk a recorder ARGS list and project every element. Used as the
   default ARG-PROJECTOR when none is specified at install time."
  (mapcar #'flatten-trace-value args))

(defun flatten-results (results)
  "Walk a recorder RESULTS list (multiple-value-list of the call) and
   project every element. Used as the default RESULT-PROJECTOR."
  (mapcar #'flatten-trace-value results))
