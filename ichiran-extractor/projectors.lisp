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
;;; per-FQN projector glue. *OMIT-SLOTS* suppresses individual
;;; slots — used today to drop ENTRY.CONTENT, the JMdict XML blob
;;; that's only consumed by admin tooling.
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
  ;; The internal POSTMODERN::DAO-COLUMN-SLOTS lists the column-mapped
  ;; slots in declaration order — postmodern itself uses it everywhere,
  ;; so the symbol is stable across releases. Non-DAO standard-objects
  ;; (closures, plain CLOS) fall through to the default method.
  (let ((cls (class-of v)))
    (if (typep cls 'postmodern:dao-class)
        (let ((cls-name (class-name cls)))
          (list* :class
                 (%class-keyword v)
                 (loop for s in (postmodern::dao-column-slots cls)
                       for sn = (closer-mop:slot-definition-name s)
                       unless (%slot-omitted-p cls-name sn)
                       collect (%slot-keyword sn)
                       and collect (flatten-trace-value (%safe-slot v sn)))))
        (call-next-method))))

(defun flatten-args (args)
  "Walk a recorder ARGS list and project every element. Used as the
   default ARG-PROJECTOR when none is specified at install time."
  (mapcar #'flatten-trace-value args))

(defun flatten-results (results)
  "Walk a recorder RESULTS list (multiple-value-list of the call) and
   project every element. Used as the default RESULT-PROJECTOR."
  (mapcar #'flatten-trace-value results))
