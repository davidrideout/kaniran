;;;; introspect.lisp
;;;;
;;;; Reverse-engineering doc generator for the ichiran codebase.
;;;;
;;;; Usage on remote (where ichiran loads cleanly):
;;;;     sbcl --non-interactive --load introspect.lisp -- /tmp/ichiran-reverse
;;;;
;;;; Output: one markdown file per function/macro/generic under
;;;;   <output-dir>/<source-basename>.lisp/<symbol-name>.md
;;;; plus <output-dir>/index.md.
;;;;
;;;; Determinism: same image, same source tree -> same output bytes.
;;;; All hash-table iterations are sorted before emitting.

(require :asdf)

;;; --- Quicklisp bootstrap ---------------------------------------------------
(unless (find-package :ql)
  (dolist (cand (list (merge-pathnames "quicklisp/setup.lisp" (user-homedir-pathname))
                      #p"/storage/quicklisp/setup.lisp"
                      #p"/root/quicklisp/setup.lisp"))
    (when (and (probe-file cand) (not (find-package :ql)))
      (load cand))))
(unless (find-package :ql)
  (error "Quicklisp not available; cannot load ichiran"))

(format t "~&;; Loading ichiran systems...~%")
(funcall (read-from-string "ql:quickload") '(:ichiran :ichiran/cli) :silent t)

(require :sb-introspect)

;;; --- Output dir ------------------------------------------------------------
(defparameter *output-dir-string*
  (let* ((argv sb-ext:*posix-argv*)
         (after (member "--" argv :test #'string=))
         (arg (cadr after)))
    (concatenate 'string (string-right-trim "/" (or arg "/tmp/ichiran-reverse")) "/")))

(defparameter *output-dir* (sb-ext:parse-native-namestring *output-dir-string*))

(defun native-path (str)
  "Parse a namestring as a literal native path (no wildcard interpretation)."
  (sb-ext:parse-native-namestring str))

(format t ";; Output dir: ~a~%" *output-dir*)

;;; --- Package / symbol universe --------------------------------------------
(defun ichiran-package-p (pkg)
  (and pkg
       (let ((n (package-name pkg)))
         (and (>= (length n) 7)
              (string-equal "ICHIRAN" n :end2 7)
              (or (= (length n) 7) (char= (char n 7) #\/))))))

(defun collect-universe ()
  "Hash-set of every symbol whose home package is an ichiran/* package."
  (let ((u (make-hash-table :test 'eq)))
    (dolist (pkg (list-all-packages))
      (when (ichiran-package-p pkg)
        (do-symbols (s pkg)
          (when (eq (symbol-package s) pkg)
            (setf (gethash s u) t)))))
    u))

(defparameter *universe* (collect-universe))
(format t ";; Universe size: ~a symbols~%" (hash-table-count *universe*))

;;; --- Classification --------------------------------------------------------
(defun classify (sym)
  (cond ((macro-function sym) :macro)
        ((not (fboundp sym)) nil)
        ((typep (fdefinition sym) 'standard-generic-function) :generic)
        ((fboundp sym) :function)
        (t nil)))

;;; --- Source resolution -----------------------------------------------------
(defparameter *file-cache* (make-hash-table :test 'equal))

(defun file-content (path)
  (let ((key (namestring path)))
    (or (gethash key *file-cache*)
        (setf (gethash key *file-cache*)
              (with-open-file (s path :direction :input
                                      :external-format :utf-8)
                (with-output-to-string (o)
                  (loop for c = (read-char s nil nil) while c do (write-char c o))))))))

(defun offset-to-line (path offset)
  (when (and path offset (probe-file path))
    (handler-case
        (let ((c (file-content path)))
          (1+ (count #\Newline c :end (min offset (length c)))))
      (error () nil))))

(defun safe-source-by-name (sym kind)
  (handler-case
      (first (sb-introspect:find-definition-sources-by-name
              sym (ecase kind
                    (:function :function)
                    (:generic :generic-function)
                    (:macro :macro))))
    (error () nil)))

(defun src-path (src) (and src (sb-introspect:definition-source-pathname src)))
(defun src-offset (src) (and src (sb-introspect:definition-source-character-offset src)))

;;; --- Lambda list / docs / type --------------------------------------------
(defun lambda-list-of (sym kind)
  (handler-case
      (case kind
        (:function (sb-introspect:function-lambda-list (fdefinition sym)))
        (:generic (sb-introspect:function-lambda-list (fdefinition sym)))
        (:macro (sb-introspect:function-lambda-list (macro-function sym))))
    (error () nil)))

(defun ftype-string (sym kind)
  (when (eq kind :function)
    (let ((ft (handler-case (sb-introspect:function-type sym) (error () nil))))
      (when (and (consp ft) (eq (first ft) 'function)
                 (not (and (eq (second ft) '*) (eq (third ft) '*))))
        ft))))

(defun docstring-of (sym kind)
  (declare (ignore kind))
  (handler-case (documentation sym 'function) (error () nil)))

(defun source-form-of (fn)
  (when (functionp fn)
    (multiple-value-bind (form closurep name)
        (function-lambda-expression fn)
      (declare (ignore closurep name))
      form)))

;;; --- Symbol-walking ---------------------------------------------------------
(defun walk-form-symbols (form)
  (let ((found (make-hash-table :test 'eq)))
    (labels ((w (x)
               (cond
                 ((symbolp x)
                  (when (gethash x *universe*) (setf (gethash x found) t)))
                 ((consp x) (w (car x)) (w (cdr x))))))
      (w form))
    (loop for k being the hash-keys of found collect k)))

;;; --- who-calls inversion ---------------------------------------------------
;;;
;;; Caller key shapes returned by sb-introspect:who-calls:
;;;   - SYMBOL                          : top-level defun/defmacro
;;;   - (:METHOD gf-name [quals...] (specializers))   : a method
;;;   - (SETF SYMBOL)                   : setf-function
;;;   - (SB-PCL::FAST-METHOD ...)       : fast-method form (treat as :method)
;;;
;;; We normalize fast-method -> :method, leave others as-is (eq/equal-keyed).

(defun normalize-caller (name)
  (cond
    ((symbolp name) name)
    ((and (consp name) (eq (car name) 'sb-pcl::fast-method))
     (cons :method (cdr name)))
    (t name)))

(defun build-call-graph ()
  "caller-key -> hash-set of callee symbols."
  (let ((g (make-hash-table :test 'equal)))
    (loop for callee being the hash-keys of *universe*
          for i from 0
          when (zerop (mod i 100)) do (format t ";;   who-calls progress: ~a~%" i)
          do (handler-case
                 (dolist (info (sb-introspect:who-calls callee))
                   (let* ((c (first info))
                          (k (normalize-caller c))
                          (set (or (gethash k g)
                                   (setf (gethash k g)
                                         (make-hash-table :test 'eq)))))
                     (setf (gethash callee set) t)))
               (error () nil)))
    g))

(defparameter *call-graph* nil)

;;; --- Method utilities ------------------------------------------------------
(defun specializer-name (s)
  (cond
    ((typep s 'class) (class-name s))
    ((typep s 'sb-mop:eql-specializer)
     (list 'eql (sb-mop:eql-specializer-object s)))
    (t s)))

(defun method-key (m)
  (let* ((gf (sb-mop:method-generic-function m))
         (gf-name (sb-mop:generic-function-name gf))
         (quals (method-qualifiers m))
         (specs (mapcar #'specializer-name (sb-mop:method-specializers m))))
    (append (list :method gf-name) quals (list specs))))

(defun method-source (m)
  (handler-case (sb-introspect:find-definition-source m) (error () nil)))

;;; --- Filename / formatting -------------------------------------------------
(defun sanitize-name (s)
  (with-output-to-string (out)
    (loop for c across s
          do (cond
               ((or (char= c #\/) (char= c #\\) (char= c #\:) (char= c #\Nul))
                (write-char #\_ out))
               (t (write-char c out))))))

(defun pkg-prefix (sym)
  (string-downcase (package-name (symbol-package sym))))

(defun fmt-sym (sym)
  (format nil "~a:~a" (pkg-prefix sym) (string-downcase (symbol-name sym))))

(defun source-basename (path)
  (when path
    (let ((n (pathname-name path))
          (e (pathname-type path)))
      (if e (format nil "~a.~a" n e) n))))

;;; --- Dependency lookup -----------------------------------------------------
(defun deps-for-key (key)
  (let* ((set (gethash key *call-graph*))
         (callees (when set
                    (loop for k being the hash-keys of set
                          collect k))))
    (sort (remove-duplicates callees :test #'eq) #'string< :key #'fmt-sym)))

(defun deps-for-symbol-with-form (sym form)
  "who-calls deps from the call graph + symbols walked from the source form
   (catches macro expansions and funcall references)."
  (let ((from-graph (deps-for-key sym))
        (from-form (when form (walk-form-symbols form))))
    (sort (remove-duplicates (append from-graph from-form) :test #'eq)
          #'string< :key #'fmt-sym)))

;;; --- Markdown writers ------------------------------------------------------
(defun escape-md (s)
  (when s
    (with-output-to-string (out)
      (loop for c across s do
        (cond ((char= c #\Newline) (write-string "  " out) (write-char #\Newline out))
              ((char= c #\`) (write-char #\' out))
              (t (write-char c out)))))))

(defun write-fn-or-macro-md (path sym kind src deps)
  (ensure-directories-exist path)
  (with-open-file (out path :direction :output
                            :if-exists :supersede :if-does-not-exist :create
                            :external-format :utf-8)
    (let* ((file (source-basename (src-path src)))
           (line (offset-to-line (src-path src) (src-offset src)))
           (ll (lambda-list-of sym kind))
           (form (when (eq kind :function)
                   (handler-case (source-form-of (fdefinition sym)) (error () nil))))
           (ft (ftype-string sym kind))
           (doc (docstring-of sym kind))
           (kw (case kind (:function "defun") (:macro "defmacro"))))
      (declare (ignore form))
      (format out "# ~a~%~%" (string-downcase (symbol-name sym)))
      (format out "**Package:** `~a`  ~%" (pkg-prefix sym))
      (when file
        (if line
            (format out "**Source:** `~a:~a`  ~%" file line)
            (format out "**Source:** `~a`  ~%" file)))
      (format out "**Definition form:** `~a`~%~%" kw)
      (format out "## Inputs~%~%")
      (cond
        (ll (format out "`~(~s~)`~%~%" ll))
        (t (format out "_(none / unable to retrieve)_~%~%")))
      (format out "## Outputs~%~%")
      (cond
        (ft (format out "Declared ftype: `~(~s~)`~%~%" ft))
        (doc (format out "Docstring: ~a~%~%" (escape-md doc)))
        (t (format out "_unknown — no declared ftype, no docstring_~%~%")))
      (format out "## Dependencies (ichiran symbols)~%~%")
      (if deps
          (dolist (d deps) (format out "- `~a`~%" (fmt-sym d)))
          (format out "_(none detected)_~%")))))

(defun write-generic-md (path sym src deps-top methods)
  (ensure-directories-exist path)
  (with-open-file (out path :direction :output
                            :if-exists :supersede :if-does-not-exist :create
                            :external-format :utf-8)
    (let* ((file (source-basename (src-path src)))
           (line (offset-to-line (src-path src) (src-offset src)))
           (ll (lambda-list-of sym :generic))
           (doc (docstring-of sym :generic)))
      (format out "# ~a (generic function)~%~%" (string-downcase (symbol-name sym)))
      (format out "**Package:** `~a`  ~%" (pkg-prefix sym))
      (when file
        (if line
            (format out "**Source:** `~a:~a`  ~%" file line)
            (format out "**Source:** `~a`  ~%" file)))
      (format out "**Definition form:** `defgeneric`~%~%")
      (format out "## Inputs (generic lambda list)~%~%")
      (cond
        (ll (format out "`~(~s~)`~%~%" ll))
        (t (format out "_(none)_~%~%")))
      (format out "## Outputs~%~%")
      (cond
        (doc (format out "Docstring: ~a~%~%" (escape-md doc)))
        (t (format out "_unknown — no docstring_~%~%")))
      (format out "## Dependencies at generic dispatch site~%~%")
      (if deps-top
          (dolist (d deps-top) (format out "- `~a`~%" (fmt-sym d)))
          (format out "_(none detected)_~%~%"))
      (format out "~%## Methods~%~%")
      (if (null methods)
          (format out "_(no methods loaded)_~%")
          (dolist (m methods)
            (let* ((msrc (method-source m))
                   (mfile (source-basename (src-path msrc)))
                   (mline (offset-to-line (src-path msrc) (src-offset msrc)))
                   (mll (sb-mop:method-lambda-list m))
                   (specs (mapcar #'specializer-name (sb-mop:method-specializers m)))
                   (quals (method-qualifiers m))
                   (mform (handler-case (source-form-of (sb-mop:method-function m))
                            (error () nil)))
                   (mkey (method-key m))
                   (form-deps (when mform (walk-form-symbols mform)))
                   (graph-deps (deps-for-key mkey))
                   (mdeps (sort (remove-duplicates (append form-deps graph-deps) :test #'eq)
                                #'string< :key #'fmt-sym)))
              (format out "### method (~{~s~^ ~})~@[ ~{~s~^ ~}~]~%~%"
                      specs quals)
              (when mfile
                (if mline
                    (format out "**Source:** `~a:~a`  ~%" mfile mline)
                    (format out "**Source:** `~a`  ~%" mfile)))
              (format out "**Inputs:** `~(~s~)`~%~%" mll)
              (format out "**Dependencies:**~%~%")
              (if mdeps
                  (dolist (d mdeps) (format out "- `~a`~%" (fmt-sym d)))
                  (format out "_(none detected)_~%"))
              (format out "~%")))))))

;;; --- Main loop -------------------------------------------------------------
(defstruct entry sym kind src filename basename methods)

(defun collect-entries ()
  (let ((entries nil))
    (loop for sym being the hash-keys of *universe* do
      (let* ((kind (classify sym))
             (src (and kind (safe-source-by-name sym kind))))
        (when (and kind src (src-path src))
          (let* ((path (src-path src))
                 (base (source-basename path)))
            (when base
              (push (make-entry :sym sym :kind kind :src src
                                :filename path :basename base
                                :methods (when (eq kind :generic)
                                           (handler-case
                                               (sb-mop:generic-function-methods (fdefinition sym))
                                             (error () nil))))
                    entries))))))
    entries))

(defun group-by-basename (entries)
  (let ((g (make-hash-table :test 'equal)))
    (dolist (e entries)
      (push e (gethash (entry-basename e) g)))
    g))

(defun emit-all (entries)
  (let ((groups (group-by-basename entries)))
    (loop for base being the hash-keys of groups
          for es = (gethash base groups) do
      (dolist (e es)
        (let* ((sym (entry-sym e))
               (kind (entry-kind e))
               (src (entry-src e))
               (fname (concatenate 'string
                                   (sanitize-name (string-downcase (symbol-name sym)))
                                   ".md"))
               (out-path (native-path
                          (concatenate 'string *output-dir-string* base "/" fname))))
          (handler-case
              (case kind
                ((:function :macro)
                 (let* ((form (when (eq kind :function)
                                (handler-case (source-form-of (fdefinition sym))
                                  (error () nil))))
                        (deps (deps-for-symbol-with-form sym form)))
                   (write-fn-or-macro-md out-path sym kind src deps)))
                (:generic
                 (let ((deps-top (deps-for-key sym)))
                   (write-generic-md out-path sym src deps-top (entry-methods e)))))
            (error (c)
              (format t ";; SKIP ~a (~a): ~a~%" sym kind c))))))
    groups))

(defun write-index (groups)
  (let ((index (native-path (concatenate 'string *output-dir-string* "index.md"))))
    (ensure-directories-exist index)
    (with-open-file (out index :direction :output
                               :if-exists :supersede :if-does-not-exist :create
                               :external-format :utf-8)
      (format out "# ichiran reverse-engineering index~%~%")
      (format out "Auto-generated by `reverse/scripts/introspect.lisp`.  ~%")
      (format out "Total source files: ~a.~%~%"
              (hash-table-count groups))
      (let ((bases (sort (loop for k being the hash-keys of groups collect k) #'string<)))
        (dolist (base bases)
          (let* ((es (sort (copy-list (gethash base groups))
                           #'string< :key (lambda (e) (string-downcase (symbol-name (entry-sym e))))))
                 (counts (make-hash-table)))
            (dolist (e es) (incf (gethash (entry-kind e) counts 0)))
            (format out "## ~a~%~%" base)
            (format out "Counts: ~{~a=~a~^, ~}~%~%"
                    (loop for k being the hash-keys of counts
                          using (hash-value v)
                          append (list (string-downcase (symbol-name k)) v)))
            (dolist (e es)
              (let* ((sym (entry-sym e))
                     (fname (concatenate 'string
                                         (sanitize-name (string-downcase (symbol-name sym)))
                                         ".md"))
                     (kindlbl (case (entry-kind e)
                                (:function "fn") (:macro "macro") (:generic "gf"))))
                (format out "- [~a](~a/~a) — ~a (`~a`)~%"
                        (string-downcase (symbol-name sym))
                        base fname kindlbl (pkg-prefix sym))))
            (format out "~%")))))))

;;; --- Run -------------------------------------------------------------------
(format t ";; Building call graph (who-calls inversion)...~%")
(setf *call-graph* (build-call-graph))
(format t ";; Call graph entries: ~a~%" (hash-table-count *call-graph*))

(format t ";; Collecting entries...~%")
(let* ((entries (collect-entries)))
  (format t ";; ~a entries to emit.~%" (length entries))
  (let ((groups (emit-all entries)))
    (write-index groups)
    (format t ";; Wrote ~a source-file directories.~%"
            (hash-table-count groups))))

(format t ";; Done. Output: ~a~%" *output-dir*)
