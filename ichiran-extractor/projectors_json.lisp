;;; projectors_json.lisp — JSON projector + encoder for the trace recorder.
;;;
;;; Sibling to projectors.lisp. Same role: walks an arbitrary Lisp value
;;; produced by the recorder and returns a primitive-printable
;;; equivalent. Differs in encoding: produces a JSON string that the
;;; Rust audit side deserializes via serde, instead of an s-expr that
;;; needs the hand-rolled CL prin1 reader on the Rust side.
;;;
;;; Encoding rules (mirror of kaniran-core/examples/common/mod.rs):
;;;   string  -> JSON string
;;;   integer -> JSON integer
;;;   float   -> JSON number
;;;   T       -> true
;;;   NIL     -> null
;;;   :KW     -> ":KW" string (leading colon distinguishes from regular strings)
;;;   :NULL   -> ":NULL" (DB-null sentinel, falls out of the keyword rule)
;;;   #\char  -> {"_meta": {"char": codepoint}}
;;;   (a . b) -> {"_meta": {"cons": [a, b]}}
;;;   (a b c) -> [a, b, c]
;;;   DAO/struct/standard-object -> {"_meta": {"class": "FOO-BAR"},
;;;                                  "slot_a": ..., "slot_b": ...}
;;;
;;; Class names stay SCREAMING-KEBAB-CASE (`KANA-TEXT`); slot keys
;;; become snake_case (`common_tags`).
;;;
;;; `*omit-slots*` is the per-class blocklist with class-precedence-list
;;; walk. Empty by default — every CLOS slot captures. The
;;; `simple-text` `conjugations` AND `hintedp` slots ARE load-bearing
;;; (audit replay surfaced both in turn):
;;;   - `conjugations` decides best-kana-conj's :root / :all / by-id arm.
;;;   - `hintedp` is the re-entrance flag the `:around get-kana` method
;;;     checks; proxy-text constructed by abbr-suffix patterns
;;;     (dict-grammar.lisp:574) is hintedp=T at runtime, and the audit
;;;     replay was misrouting those rows through the hint branch.
;;; Re-extract any parquet that depends on hintedp accuracy.
;;;
;;; Exposes:
;;;   FLATTEN-TO-JSON value      -> jsown-compatible tree
;;;   FLATTEN-ARGS-JSON args     -> tree (for trace_capture arg-projector)
;;;   FLATTEN-RESULTS-JSON list  -> tree (for trace_capture result-projector)
;;;   ENCODE-JSON tree           -> string (trace_capture encoder when :encoder :json)

(defpackage :ichi-projectors-json
  (:use :cl)
  (:export #:flatten-to-json
           #:flatten-args-json
           #:flatten-args-json-with-hint-state
           #:flatten-results-json
           #:encode-json
           #:to-json-string
           #:*omit-slots*))

(in-package :ichi-projectors-json)

(defparameter *omit-slots*
  '((ichiran/dict::entry ichiran/dict::content))
  "Per-class slot blocklist. Each entry: (CLASS-NAME SLOT-NAME ...). The
   default flatten skips listed slots when expanding object plists.
   Slot lookup walks the class-precedence-list, so listing a slot on an
   ancestor class covers all subclasses.

   `entry.content` stays omitted because it's the full JMdict XML
   payload (kilobytes per row); audit replay doesn't need it.
   Everything else — including `simple-text.{conjugations, hintedp}` —
   captures by default.")

(defun %slot-omitted-p (cls slot-name)
  (some (lambda (ancestor)
          (let ((ancestor-name (class-name ancestor)))
            (some (lambda (entry)
                    (and (eq (first entry) ancestor-name)
                         (member slot-name (rest entry))))
                  *omit-slots*)))
        (closer-mop:class-precedence-list cls)))

(defun %safe-slot (instance slot-name)
  (handler-case (and (slot-boundp instance slot-name)
                     (slot-value instance slot-name))
    (error () nil)))

(defun %slot-key (slot-name)
  ;; Slot symbol UPPER-KEBAB-CASE -> snake_case JSON key.
  (let ((source (symbol-name slot-name)))
    (with-output-to-string (out)
      (loop for ch across source do
            (write-char (cond ((char= ch #\-) #\_)
                              (t (char-downcase ch)))
                        out)))))


;; --- generic walk: Lisp value -> tagged tree -------------------------------

(defgeneric flatten-to-json (v))
(defmethod flatten-to-json (v) v)
(defmethod flatten-to-json ((v null)) nil)
(defmethod flatten-to-json ((v (eql t))) t)
(defmethod flatten-to-json ((v symbol))
  (cond ((eq v t) t) ((null v) nil)
        (t (format nil ":~A" (symbol-name v)))))
(defmethod flatten-to-json ((v character))
  `(:obj ("_meta" . (:obj ("char" . ,(char-code v))))))
(defmethod flatten-to-json ((v cons))
  (cond ((null (cdr (last v)))
         (mapcar #'flatten-to-json v))
        (t `(:obj ("_meta" . (:obj ("cons" . (,(flatten-to-json (car v))
                                              ,(flatten-to-json (cdr v))))))))))
(defmethod flatten-to-json ((v vector))
  (if (stringp v) v (map 'list #'flatten-to-json v)))

(defmethod flatten-to-json ((v structure-object))
  (let* ((cls (class-of v))
         (cls-name (class-name cls)))
    `(:obj ("_meta" . (:obj ("class" . ,(symbol-name cls-name))))
           ,@(loop for s in (closer-mop:class-slots cls)
                   for sn = (closer-mop:slot-definition-name s)
                   unless (%slot-omitted-p cls sn)
                   collect (cons (%slot-key sn)
                                 (flatten-to-json (%safe-slot v sn)))))))

(defmethod flatten-to-json ((v standard-object))
  (let ((cls (class-of v)))
    (cond
      ((typep cls 'postmodern:dao-class)
       (let* ((cls-name (class-name cls))
              (cols (postmodern::dao-column-slots cls))
              (col-names (mapcar #'closer-mop:slot-definition-name cols))
              (extras (remove-if (lambda (s) (member (closer-mop:slot-definition-name s) col-names))
                                 (closer-mop:class-slots cls))))
         `(:obj ("_meta" . (:obj ("class" . ,(symbol-name cls-name))))
                ,@(loop for s in cols for sn = (closer-mop:slot-definition-name s)
                        unless (%slot-omitted-p cls sn)
                        collect (cons (%slot-key sn) (flatten-to-json (%safe-slot v sn))))
                ,@(loop for s in extras for sn = (closer-mop:slot-definition-name s)
                        unless (%slot-omitted-p cls sn)
                        collect (cons (%slot-key sn) (flatten-to-json (%safe-slot v sn)))))))
      ((typep cls 'standard-class)
       (let ((cls-name (class-name cls)))
         `(:obj ("_meta" . (:obj ("class" . ,(symbol-name cls-name))))
                ,@(loop for s in (closer-mop:class-slots cls)
                        for sn = (closer-mop:slot-definition-name s)
                        unless (%slot-omitted-p cls sn)
                        collect (cons (%slot-key sn) (flatten-to-json (%safe-slot v sn)))))))
      (t (call-next-method)))))

(defun flatten-args-json (args) (mapcar #'flatten-to-json args))
(defun flatten-results-json (results) (mapcar #'flatten-to-json results))

;; Hint-aware variant — appends `*disable-hints*` value as a trailing
;; meta-tagged entry so audit-replay can distinguish hint-active calls
;; from `:around`-disabled ones. The dict-split.lisp:80 `:around` method
;; binds `*disable-hints*` to T before calling `get-hint`, so capturing
;; it here is the only way to disambiguate the ~5% output variation that
;; the segmenter extraction documented.
;;
;; Trailing element shape: {"_meta":{"context":{"disable_hints":<bool>}}}
;; — audit handlers detect it structurally and split rows by binding state.
(defun flatten-args-json-with-hint-state (args)
  (let ((disable-hints (symbol-value (find-symbol "*DISABLE-HINTS*" :ichiran/dict))))
    (append (mapcar #'flatten-to-json args)
            (list `(:obj ("_meta" . (:obj ("context" . (:obj ("disable_hints" . ,(if disable-hints t nil)))))))))))


;; --- JSON encoder ----------------------------------------------------------
;;
;; Direct emit of the tagged tree. NIL -> null (where jsown emits []).
;; Atoms straight through; strings escaped per RFC 8259.

(defun %emit-json-string (str out)
  (write-char #\" out)
  (loop for ch across str do
        (let ((code (char-code ch)))
          (cond
            ((char= ch #\") (write-string "\\\"" out))
            ((char= ch #\\) (write-string "\\\\" out))
            ((char= ch #\Newline) (write-string "\\n" out))
            ((char= ch #\Tab) (write-string "\\t" out))
            ((char= ch #\Return) (write-string "\\r" out))
            ((< code 32) (format out "\\u~4,'0X" code))
            (t (write-char ch out)))))
  (write-char #\" out))

(defun encode-json (val out)
  (cond
    ((null val) (write-string "null" out))
    ((eq val t) (write-string "true" out))
    ((stringp val) (%emit-json-string val out))
    ((integerp val) (princ val out))
    ((numberp val) (princ val out))
    ((and (consp val) (eq (car val) :obj))
     (write-char #\{ out)
     (loop for (key . value) in (cdr val) for first = t then nil do
           (unless first (write-char #\, out))
           (%emit-json-string key out)
           (write-char #\: out)
           (encode-json value out))
     (write-char #\} out))
    ((listp val)
     (write-char #\[ out)
     (loop for x in val for first = t then nil do
           (unless first (write-char #\, out))
           (encode-json x out))
     (write-char #\] out))
    (t (error "can't emit JSON for: ~A (type ~A)" val (type-of val)))))

(defun to-json-string (val)
  "Convert VAL to a JSON string. Used by trace_capture's recorder when
   the installed FQN was configured with :encoder :json."
  (with-output-to-string (out) (encode-json val out)))
