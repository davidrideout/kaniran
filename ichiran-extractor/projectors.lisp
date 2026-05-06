;;; projectors.lisp — named result projectors for the trace recorder.
;;;
;;; Loaded by extractor_worker.lisp after trace_capture.lisp. Defines
;;; the :ICHI-PROJECTORS package; each exported symbol is a function
;;; suitable as the :RESULT-PROJECTOR argument to ICHI-TRACE:INSTALL.
;;;
;;; A projector takes RESULTS — the multiple-value-list of the original
;;; call's return — and returns a primitive-printable equivalent. The
;;; recorder calls SAFE-PRIN1 on the projector's output instead of the
;;; raw RESULTS, so DAO instances and other non-readable types can be
;;; flattened into column-tuple plists per-FQN before capture.
;;;
;;; If a projector raises an error, the recorder counts the call as
;;; skipped (same path as a primitive-gate failure).
;;;
;;; The wire protocol resolves projectors by name: the install op
;;; accepts JSON objects of shape {"fqn":"PKG:SYM","result_projector":
;;; "PROJECTOR-NAME"}, and the worker calls FIND-SYMBOL on the
;;; uppercased name in :ICHI-PROJECTORS to get the function.

(defpackage :ichi-projectors
  (:use :cl)
  (:export #:dao-rows
           #:dao-row-or-nil))

(in-package :ichi-projectors)

(defun dao-row (dao)
  "Project a single ICHIRAN/DICT KANJI-TEXT or KANA-TEXT instance to
   its identifying column-tuple plist. :CLASS is a keyword so SBCL
   prints it as :KANJI-TEXT / :KANA-TEXT instead of falling back to
   the #A(...) simple-base-string array syntax that *print-readably*
   forces on plain symbol-name strings."
  (let* ((cls (class-name (class-of dao)))
         (cls-key (intern (symbol-name cls) :keyword)))
    (list :class cls-key
          :id    (ichiran/dict::id   dao)
          :seq   (ichiran/dict::seq  dao)
          :text  (ichiran/dict::text dao)
          :ord   (ichiran/dict::ord  dao))))

(defun dao-rows (results)
  "Project a single-value return shape ((list-of-DAOs)) to
   ((list-of-plists)). Use for fns that return a list of KANJI-TEXT
   or KANA-TEXT rows — find-word-seq, find-word-conj-of, and the rest
   of the dict-grammar finders that share the polymorphic dispatch on
   (test-word word :kana)."
  (list (mapcar #'dao-row (first results))))

(defun dao-row-or-nil (results)
  "Project a single-value return shape ((DAO-or-NIL)) to ((plist-or-NIL))
   for fns that return one DAO or nil. Used by get-kana-form."
  (let ((dao (first results)))
    (list (and dao (dao-row dao)))))
