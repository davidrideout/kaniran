;;; Probe — installs the 5 next-10 PORT_PLAN callables with the
;;; package-default flatten (no per-FQN projector glue), drives
;;; tatoeba_2000.txt through ichiran:romanize*, and reports per-fn
;;; captures / skips plus a non-trivial sample for each.
(in-package :cl-user)
(load "/home/david/pooled-api/trace_capture.lisp")
(load "/home/david/pooled-api/projectors.lisp")

(defparameter *targets*
  '("ICHIRAN/DICT:NO-CONJ-DATA"
    "ICHIRAN/DICT:MAKE-CONJ-DATA"
    "ICHIRAN/DICT:GET-CONJ-DATA"
    "ICHIRAN/DICT:CONJ-DATA-PROP"
    "ICHIRAN/DICT:TEST-CONJ-PROP"))

(dolist (fqn *targets*) (ichi-trace:install fqn))
(format t "INSTALLED: ~a~%" (ichi-trace:installed))
(finish-output)

(defparameter *corpus-path* "/home/david/storage/tatoeba_2000.txt")
(defparameter *sentences*
  (with-open-file (in *corpus-path* :direction :input :external-format :utf-8)
    (loop for line = (read-line in nil nil)
          while line
          when (> (length (string-trim '(#\Space #\Tab #\Return) line)) 0)
          collect line)))
(format t "CORPUS: ~a sentences~%" (length *sentences*)) (finish-output)

(defparameter *progress-every* 200)
(defparameter *t0* (get-internal-real-time))
(defun elapsed-secs ()
  (/ (- (get-internal-real-time) *t0*)
     (float internal-time-units-per-second)))

(let ((errors 0)
      (n (length *sentences*)))
  (loop for s in *sentences*
        for i from 1
        do (handler-case
               (let ((r (ichiran:romanize* s)))
                 (declare (ignore r)))
             (error () (incf errors)))
        when (zerop (mod i *progress-every*))
        do (let* ((dt (elapsed-secs))
                  (rate (if (zerop dt) 0 (/ i dt))))
             (format t "  progress ~a/~a  elapsed=~,1fs  rate=~,1f sent/s  errors=~a  caps=~a  skip=~a~%"
                     i n dt rate errors
                     (ichi-trace:n-captures) (ichi-trace:n-skipped))
             (finish-output)))
  (format t "DONE  total=~a  errors=~a  total-elapsed=~,1fs~%"
          n errors (elapsed-secs)))
(finish-output)

(format t "TOTAL-CAPTURES: ~a~%" (ichi-trace:n-captures))
(format t "TOTAL-SKIPPED:  ~a~%" (ichi-trace:n-skipped))

(let ((per-fqn (make-hash-table :test 'equal)))
  (dolist (cap (ichi-trace:captures))
    (incf (gethash (first cap) per-fqn 0)))
  (dolist (fqn *targets*)
    (format t "  ~a captures=~a~%" fqn (gethash fqn per-fqn 0))))
(finish-output)

(let ((picked (make-hash-table :test 'equal))
      (caps   (reverse (ichi-trace:captures))))
  (flet ((pick (fqn nontrivial)
           (loop for c in caps
                 when (and (string= (first c) fqn)
                           (or (not nontrivial)
                               (and (not (string= (third c) "(NIL)"))
                                    (not (string= (third c) "NIL")))))
                 return c)))
    (dolist (fqn *targets*)
      (let ((cap (or (pick fqn t) (pick fqn nil))))
        (when (and cap (not (gethash fqn picked)))
          (setf (gethash fqn picked) t)
          (format t "SAMPLE ~a~%  ARGS=~a~%  RESULT=~a~%"
                  fqn
                  (subseq (second cap) 0 (min 320 (length (second cap))))
                  (subseq (third cap)  0 (min 400 (length (third cap)))))))))
  (finish-output))

;; Confirm entry.content is never present anywhere in captured ARGS/RESULT.
;; If the omit-slot rule is active the search returns 0 hits.
(let ((with-content
        (loop for c in (ichi-trace:captures)
              when (or (search ":CONTENT \"" (second c))
                       (search ":CONTENT \"" (third c)))
              count 1)))
  (format t "CAPTURES-WITH-ENTRY-CONTENT: ~a~%" with-content))
(finish-output)
(sb-ext:exit :code 0)
