;;; capture_init_fixtures.lisp — one-shot capture of fns whose only
;;; callsites live in ichiran/dict:init-suffixes (load-time setup).
;;;
;;; Wave 112 (get-kana-form) is the canonical case: ~25 deterministic
;;; calls inside load-kf, never fired by runtime romanize*. The
;;; corpus-driven extractor pool can't see them because the pool's own
;;; init runs before any /install request lands. Wave 122-150 added
;;; get-kana-forms (125), get-kana-forms* (124), and
;;; get-kana-forms-conj-data-filter (123) — same path: every load-conjs
;;; callsite drives one get-kana-forms call, which fans out into the
;;; other two.
;;;
;;; Usage:
;;;   sbcl --core ichiran.core --noinform --non-interactive \
;;;        --load capture_init_fixtures.lisp
;;;
;;; Emits one JSON object on stdout (last line):
;;;   {"captures":[{"fn":..,"args":..,"result":..}, ...],"skipped":N}
;;;
;;; Identical envelope shape to the worker pool's /extract response,
;;; so capture_init_fixtures.py can write parquet via the same machinery
;;; the corpus path uses.

(load (merge-pathnames "trace_capture.lisp" *load-pathname*))
(load (merge-pathnames "projectors.lisp" *load-pathname*))

;; Default projectors (T = use ichi-projectors:flatten-trace-value)
;; cover DAOs and structs uniformly since the 2026-05-05 refactor of
;; projectors.lisp; no per-FQN projector glue needed.
(ichi-trace:install-many
 '("ICHIRAN/DICT::GET-KANA-FORM"
   "ICHIRAN/DICT::GET-KANA-FORMS"
   "ICHIRAN/DICT::GET-KANA-FORMS*"
   "ICHIRAN/DICT::GET-KANA-FORMS-CONJ-DATA-FILTER"))

(handler-case
    (postmodern:with-connection ichiran/conn:*connection*
      ;; Blocking + reset = synchronous re-run of every load-kf form
      ;; (calls get-kana-form once per registered suffix). The reset is
      ;; required because the ichiran.core image already has
      ;; *suffix-cache* populated from build-time init, and the default
      ;; `(when (not *suffix-cache*) ...)` guard skips work otherwise.
      (ichiran/dict:init-suffixes t t))
  (error (e)
    (format *error-output* "init-suffixes failed: ~a~%" e)
    (sb-ext:exit :code 1)))

;; Emit a single JSON line so the python driver only has to parse one
;; envelope. We print the literal JSON ourselves rather than reach for
;; jsown — the values are already JSON-safe (FQNs, escaped lisp source).
(let* ((caps (ichi-trace:captures))
       (skipped (ichi-trace:n-skipped)))
  (format *error-output* "~a captures, ~a skipped~%" (length caps) skipped)
  (write-string "{\"captures\":[")
  (loop for (fn args-str result-str) in caps
        for first = t then nil
        do (unless first (write-string ","))
           (format t "{\"fn\":~a,\"args\":~a,\"result\":~a}"
                   (jsown:to-json fn)
                   (jsown:to-json args-str)
                   (jsown:to-json result-str)))
  (format t "],\"skipped\":~a}~%" skipped)
  (finish-output))
