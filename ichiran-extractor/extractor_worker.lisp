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
(load (merge-pathnames "projectors_json.lisp" *load-pathname*))


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
   or a JSON object {fqn, arg_projector?, result_projector?}. Missing
   or empty projector fields fall back to the package default (the
   ICHI-PROJECTORS generic flatten that handles DAOs and structures
   uniformly). Set a field to the literal string 'none' to disable
   projection on that side and capture the raw value."
  (labels ((resolve (name)
             (cond
               ((null name) t)
               ((not (stringp name)) t)
               ((zerop (length name)) t)
               ((string-equal name "none") nil)
               (t (resolve-projector name)))))
    (mapcar (lambda (item)
              (cond
                ((stringp item) item)
                ((and (listp item) (eq (car item) :obj))
                 (let ((fqn (jsown:val item "fqn"))
                       (arg-name (jsown:val-safe item "arg_projector"))
                       (res-name (jsown:val-safe item "result_projector")))
                   (list fqn
                         :arg-projector    (resolve arg-name)
                         :result-projector (resolve res-name))))
                (t (error "bad install spec: ~a" item))))
            raw)))


;; --- /extract handler ------------------------------------------------------

(defun handle-extract (text)
  "Run the entry-point sweep on TEXT and emit a JSON response.

   Wire-protocol robustness: ichiran's runtime path is not 100% silent —
   isolated `format t` callsites (loaders, lookup-table warnings, etc.)
   could leak bytes into *standard-output*, which IS the worker's pipe
   back to the FastAPI pool. Any rogue byte before the response would
   present in the pool as a `json.JSONDecodeError` (commonly
   `Extra data: line 1 column 5 (char 4)` when a bare `true` slips out).
   We rebind *standard-output* to a discarding broadcast stream around
   the work + JSON build, then restore the real fd for the actual
   json-ok / json-error wire write.

   Heap-pressure reclaim: a single long sentence + 13 installed FQNs
   peaks the heap at 1–2 GB (captures buffer + serialized JSON string +
   intermediate plist tree). Without an explicit major GC between
   sentences, fragmentation in gen3/gen6 makes contiguous-page
   allocation fail mid-serialize → heap-exhausted abort → pool sees
   `Worker N did not respond`. Chunk-B (2026-05-14) hit this on
   1568/250000 sentences (0.63%)."
  (cond
    ((or (null text) (zerop (length text)))
     (json-error "missing 'text'"))
    (t
     (let ((real-out *standard-output*))
       (handler-case
           (let ((payload
                  (let ((*standard-output* (make-broadcast-stream)))
                    (postmodern:with-connection ichiran/conn:*connection*
                      (call-entries text)
                      ;; Read skipped BEFORE drain — drain resets *skipped* to 0.
                      (let* ((skipped (ichi-trace:n-skipped))
                             (caps    (ichi-trace:drain)))
                        (captures-to-json caps skipped))))))
             (let ((*standard-output* real-out))
               (json-ok payload)))
         (error (e)
           (let ((*standard-output* real-out))
             (json-error (format nil "~a" e))))))
     ;; Force a major GC between sentences. Empirically, without this the
     ;; worker dies of "Heap exhausted" partway through a corpus run at
     ;; ~46% of dynamic-space-size: each /extract allocates 100MB–2GB of
     ;; transient cons cells (captures + plist tree + serialized JSON
     ;; string), and gen3 fills with retained cells faster than minor
     ;; GCs can compact. Cost is ~50–200ms per call. Trade-off codified
     ;; against chunk-B 1568/250000 heap deaths (2026-05-14).
     (sb-ext:gc :gen 2))))


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
                  (handle-extract (jsown:val-safe query "text")))

                 (t (json-error (format nil "unknown op: ~a" op))))))))))))

;; Switch *connection* into pooled mode so every internal
;; (with-connection *connection* ...) callsite in ichiran reuses this
;; worker's backend instead of churning a fresh TCP connection per
;; call. Without this each /extract burns through ~10 ephemeral ports
;; into TIME_WAIT and grows the pg backend count to ~2× the worker
;; count; with it, single-threaded SBCL → pool size 1 → exactly one
;; persistent backend per worker. Member-check guard makes the boot
;; idempotent if the worker script gets reloaded into a live image.
(unless (member :pooled-p ichiran/conn:*connection*)
  (setf ichiran/conn:*connection*
        (append ichiran/conn:*connection* '(:pooled-p t))))

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

;; Boot-time tracer install. Workers come up with these FQNs already
;; hooked so a worker crash + pool respawn does not silently drop
;; instrumentation mid-run. Without this, install state was per-image
;; and lost on respawn — replacements came up clean.
;; Each entry is either a bare FQN string (uses defaults: package-level
;; flatten projectors + :sexp encoder) or a list `(FQN &key arg-projector
;; result-projector encoder)` passed to ICHI-TRACE:INSTALL via APPLY.
;;
;; All entries use the JSON projector + JSON encoder so the resulting
;; parquet feeds straight into `kaniran-core/audit/<pkg>/<fqn>_test.rs`
;; runners.

;; Extend *omit-slots* to break the segment-list / segment `top` ->
;; top-array -> top-array-item.payload -> segment-list cycle. Without
;; this, installing WORD-INFO-FROM-SEGMENT-LIST or FILL-SEGMENT-PATH
;; crashes the projector on its first /extract call (see
;; feedback_segment_list_projector_recursion). Empirically `top` is the
;; only cyclic slot — segments / matches / start / end serialize fine.
(setf ichi-projectors-json:*omit-slots*
      (append ichi-projectors-json:*omit-slots*
              '((ichiran/dict::segment-list ichiran/dict::top)
                (ichiran/dict::segment       ichiran/dict::top)
                ;; Romanize methods (generic-romanization + subclasses)
                ;; carry a kana-table slot that holds a hash-table. SBCL
                ;; hash-tables are structure-objects, so flatten-to-json
                ;; walks their internal slots and hits a GETHASH/EQL
                ;; function pointer that encode-json can't serialize.
                ;; The class name (carried in the projected `_meta.class`)
                ;; uniquely identifies the method — the table contents
                ;; are reconstructable from class-name alone.
                (ichiran::generic-romanization ichiran::kana-table))))

;; chunk-d synergies + segfilters + penalties (2026-05-16). 38 FQNs from
;; docs/extraction-candidates/chunk-d-synergy-segfilter-penalty.md.
;;
;; Batch D1 (21 FQNs) was originally a single run but the combined per-sentence
;; payload tripped aiohttp's chunk-size limit and OOMed the wire — see
;; chunk_d1_bulk.log for the cascade. Split into D1a / D1b. Run D1a first,
;; then swap the install-set for D1b and redeploy.
(defparameter *boot-install-fqns*
  (let ((arg-fn    (symbol-function (find-symbol "FLATTEN-ARGS-JSON"
                                                 :ichi-projectors-json)))
        (result-fn (symbol-function (find-symbol "FLATTEN-RESULTS-JSON"
                                                 :ichi-projectors-json))))
    (mapcar (lambda (fqn)
              (list fqn
                    :arg-projector arg-fn
                    :result-projector result-fn
                    :encoder :json))
            ;; Gap-suffix batch (2026-05-23): the 11 suffix/abbr fns that had
            ;; zero or thin productive captures in diverse_250k. Re-extract
            ;; over the 2,342 appended gap sentences (corpus rows >= 250000).
            '("ICHIRAN/DICT:SUFFIX-SOU"
              "ICHIRAN/DICT:SUFFIX-KUDASAI"
              "ICHIRAN/DICT:SUFFIX-DESHO"
              "ICHIRAN/DICT:SUFFIX-DESU"
              "ICHIRAN/DICT:ABBR-MEBA"
              "ICHIRAN/DICT:ABBR-KEBA"
              "ICHIRAN/DICT:ABBR-GEBA"
              "ICHIRAN/DICT:ABBR-SEBA"
              "ICHIRAN/DICT:ABBR-TEBA"
              "ICHIRAN/DICT:ABBR-NEBA"
              "ICHIRAN/DICT:ABBR-REBA")
            )))

(handler-case
    (ichi-trace:install-many *boot-install-fqns*)
  (error (e)
    (format *error-output* "warn: boot install-many failed: ~a~%" e)))

(extractor-worker-loop)
