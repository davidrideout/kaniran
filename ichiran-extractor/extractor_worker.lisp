;;; extractor_worker.lisp — pooled-API worker for per-fn fixture capture.
;;;
;;; Loaded by the FastAPI service (see ichiran_main_pooled.py). Reads
;;; one JSON line per request from stdin, dispatches on "op", writes
;;; one JSON line per response to stdout. The pool keeps N of these
;;; running so the ~30s SBCL boot is paid once per pool.
;;;
;;; Boot:
;;;   sbcl --core ichiran.core --noinform --non-interactive \
;;;        --load extractor_worker.lisp
;;;
;;; Protocol:
;;;   {"op":"ping"}                         -> {"ok":true,"result":"pong"}
;;;   {"op":"quit"}                         -> exits
;;;   {"op":"installed"}                    -> {"ok":true,"result":["FQN", ...]}
;;;   {"op":"install","fqns":[SPEC, ...]}   -> {"ok":true,"result":N}
;;;     SPEC = "FQN" | {"fqn":"FQN","result_projector":"NAME"}
;;;     PROJECTOR-NAME resolves via :ichi-projectors (see projectors.lisp).
;;;   {"op":"uninstall-all"}                -> {"ok":true,"result":true}
;;;   {"op":"clear"}                        -> {"ok":true,"result":true}
;;;   {"op":"extract","text":"..."}         -> {"ok":true,"result":{
;;;                                              "captures":[{"fn":..., "args":..., "result":...}, ...],
;;;                                              "skipped":N}}
;;;
;;; "extract" runs the hardcoded ENTRY-POINTS list on the given text and
;;; returns every capture accumulated since the last extract/clear.
;;; Internal calls into other installed fns are captured for free via
;;; the encapsulate hooks.

(in-package :cl-user)

(load (merge-pathnames "trace_capture.lisp" *load-pathname*))
(load (merge-pathnames "projectors.lisp" *load-pathname*))


;; --- entry-point sweep -----------------------------------------------------

;; Functions called per /extract request. Adjust this list to widen the
;; subsystem under capture; redeploy and restart the pool to apply.
;; The entries listed here exercise the 'characters' preprocessing
;; pipeline; their internal calls into other installed fns get captured
;; via the encapsulate hooks regardless.
(defparameter *entry-points*
  '("ICHIRAN:ROMANIZE*"))

(defun call-entry (fqn text)
  "Invoke FQN with TEXT as its single argument. Wrapped in
   handler-case so a single ungraceful entry doesn't abort the
   sweep — the request still returns whatever captures landed."
  (handler-case
      (funcall (ichi-trace::resolve-symbol fqn) text)
    (error (e)
      (declare (ignore e))
      nil)))

(defun call-entries (text)
  "Call each entry-point on TEXT. Special-case the few entries
   whose arity isn't (string)."
  (loop for fqn in *entry-points*
        do (cond
             ((string-equal fqn "ICHIRAN/CHARACTERS:NORMALIZE")
              (handler-case
                  (funcall (ichi-trace::resolve-symbol fqn) text :context :default)
                (error () nil)))
             ((string-equal fqn "ICHIRAN/CHARACTERS:SEQUENTIAL-KANJI-POSITIONS")
              (handler-case
                  (funcall (ichi-trace::resolve-symbol fqn) text 0)
                (error () nil)))
             (t (call-entry fqn text)))))


;; --- JSON wire protocol ----------------------------------------------------

(defun json-error (msg)
  (format t "{\"ok\":false,\"error\":~a}~%" (jsown:to-json msg))
  (finish-output))

(defun json-ok (json-string)
  (format t "{\"ok\":true,\"result\":~a}~%" json-string)
  (finish-output))

(defun json-ok-value (value)
  (json-ok (jsown:to-json value)))

(defun parse-query (line)
  (handler-case (jsown:parse line)
    (error () nil)))

(defun captures-to-json (caps skipped)
  "Build the response payload for /extract. CAPS is the chronological
   list of (fqn args-str result-str) triples drained from the tracer;
   SKIPPED is the count of unprintable calls."
  (jsown:to-json
   (jsown:new-js
     ("captures"
      (mapcar (lambda (c)
                (jsown:new-js
                  ("fn" (first c))
                  ("args" (second c))
                  ("result" (third c))))
              caps))
     ("skipped" skipped))))


;; --- install spec translation ---------------------------------------------

(defun resolve-projector (name)
  "Resolve a projector name string to its function in :ichi-projectors.
   Errors if the symbol is missing or unbound — the worker surfaces
   that as a JSON error to the install caller."
  (let ((sym (find-symbol (string-upcase name) :ichi-projectors)))
    (unless (and sym (fboundp sym))
      (error "unknown projector: ~a" name))
    (symbol-function sym)))

(defun translate-install-specs (raw)
  "Translate the install op's 'fqns' JSON array into specs accepted by
   ichi-trace:install-many. Each element is either a JSON string FQN
   (no projector) or a JSON object {fqn, result_projector}. Empty or
   missing result_projector means no projector (raw RESULTS captured)."
  (mapcar (lambda (item)
            (cond
              ((stringp item) item)
              ((and (listp item) (eq (car item) :obj))
               (let* ((fqn (jsown:val item "fqn"))
                      (proj-name (jsown:val-safe item "result_projector")))
                 (if (and proj-name (stringp proj-name)
                          (not (zerop (length proj-name))))
                     (list fqn :result-projector (resolve-projector proj-name))
                     (list fqn))))
              (t (error "bad install spec: ~a" item))))
          raw))


;; --- dispatch loop ---------------------------------------------------------

(defun extractor-worker-loop ()
  ;; Silence stderr — broken-pipe noise on shutdown bleeds into the
  ;; pool's worker logs otherwise.
  (setf *error-output* (open "/dev/null" :direction :output :if-exists :overwrite))
  (loop
    (let ((line (read-line *standard-input* nil :eof)))
      (when (eq line :eof) (sb-ext:exit :code 0))
      (when (and (stringp line)
                 (> (length (string-trim '(#\Space #\Tab) line)) 0))
        (let ((query (parse-query line)))
          (cond
            ((null query) (json-error "invalid JSON"))

            (t
             (let ((op (jsown:val-safe query "op")))
               (cond
                 ((string-equal op "ping")
                  (json-ok "\"pong\""))

                 ((string-equal op "quit")
                  (sb-ext:exit :code 0))

                 ((string-equal op "installed")
                  (json-ok-value (ichi-trace:installed)))

                 ((string-equal op "install")
                  (let ((raw (jsown:val-safe query "fqns")))
                    (handler-case
                        (let ((specs (translate-install-specs raw)))
                          (ichi-trace:install-many specs)
                          (json-ok-value (length (ichi-trace:installed))))
                      (error (e) (json-error (format nil "~a" e))))))

                 ((string-equal op "uninstall-all")
                  (ichi-trace:uninstall-all)
                  (json-ok-value t))

                 ((string-equal op "clear")
                  (ichi-trace:clear)
                  (json-ok-value t))

                 ((string-equal op "extract")
                  (let ((text (jsown:val-safe query "text")))
                    (cond
                      ((or (null text) (zerop (length text)))
                       (json-error "missing 'text'"))
                      (t
                       (handler-case
                           (postmodern:with-connection ichiran/conn:*connection*
                             (call-entries text)
                             (let ((caps (ichi-trace:drain))
                                   (skipped (ichi-trace:n-skipped)))
                               (json-ok (captures-to-json caps skipped))))
                         (error (e) (json-error (format nil "~a" e)))))) ))

                 (t (json-error (format nil "unknown op: ~a" op))))))))))))

;; Force suffix-cache + suffix-class to populate synchronously before
;; we start accepting requests. Without this, ichiran/dict:init-suffixes
;; would race the first /extract calls (it spawns a background loader
;; thread by default), and find-word-seq calls reached via the suffix
;; matcher would slip past unhooked sentences. Blocking-init is a
;; one-time ~200ms hit per worker at boot.
(handler-case
    (postmodern:with-connection ichiran/conn:*connection*
      (ichiran/dict:init-suffixes t))
  (error (e)
    (format *error-output* "warn: init-suffixes failed: ~a~%" e)))

(extractor-worker-loop)
