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

(defun source-line-by-name (path sym-name def-keywords)
  "Fallback line lookup. SBCL does not record character offsets for class
   definitions (only form-path / form-number), so for defstruct/defclass we
   scan the source file for the first line matching `(<keyword> <name>` or
   `(<keyword> (<name>` (column 0). Linear scan; only used for the 53
   class/struct definitions, so not hot."
  (when (and path (probe-file path) sym-name def-keywords)
    (handler-case
        (with-open-file (s path :direction :input :external-format :utf-8)
          (let ((needle (string-downcase sym-name))
                (prefixes (mapcar (lambda (kw) (format nil "(~a " (string-downcase kw)))
                                  def-keywords))
                (line 0))
            (loop for raw = (read-line s nil nil)
                  while raw
                  do (incf line)
                     (let ((dl (string-downcase raw)))
                       (when (some (lambda (p)
                                     (and (>= (length dl) (length p))
                                          (string= p dl :end2 (length p))
                                          (or (search (concatenate 'string p needle) dl)
                                              (search (concatenate 'string p "(" needle) dl))))
                                   prefixes)
                         (return line))))))
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

;;; --- Source-walk pass ------------------------------------------------------
;;;
;;; Runtime introspection misses dependency edges that only appear at the
;;; source level — most importantly DAO-class symbols passed as data to
;;; postmodern helpers like (select-dao 'kana-text ...). The runtime call
;;; graph only sees function calls; quoted symbols are data, not calls.
;;;
;;; This pass re-reads each ichiran .lisp source file with the Lisp reader,
;;; identifies every top-level defining form, and walks the form for
;;; ichiran-package symbols. Output: a hash table keyed by defining symbol
;;; with the set of ichiran symbols mentioned anywhere in its body. The md
;;; writers consult this table to emit a "## Source-walked references"
;;; section. build_graph.py reads that section and unions it with the
;;; runtime call-graph deps before writing edges.csv. Symbols that are not
;;; top-level definitions (locals, lambda-list parameters, slot names) get
;;; filtered out on the Python side via cross-reference with symbols.csv.

(defparameter *defining-head-names*
  '("DEFUN" "DEFMACRO" "DEFMETHOD" "DEFGENERIC" "DEFCLASS" "DEFSTRUCT"
    "DEFINE-CONDITION" "DEFTYPE" "DEFPARAMETER" "DEFVAR" "DEFCONSTANT"))

(defparameter *source-walk-deps* (make-hash-table :test 'eq)
  "Defining-symbol -> hash-set of ichiran-package symbols seen in its form.
   Methods are aggregated under the GF symbol — the runtime pass already
   gives precise per-method edges, and the source pass complements it at
   the GF level.")

(defun defining-symbol-of-form (form)
  "Return the ichiran symbol defined by FORM, or NIL.
   Handles `(defstruct (name options) ...)` plus the common
   `(defXXX name ...)` shape."
  (when (and (consp form)
             (symbolp (car form))
             (member (symbol-name (car form))
                     *defining-head-names* :test #'string=)
             (consp (cdr form)))
    (let ((second (cadr form)))
      (cond
        ((and (symbolp second)
              (symbol-package second)
              (ichiran-package-p (symbol-package second)))
         second)
        ((and (consp second)
              (symbolp (car second))
              (symbol-package (car second))
              (ichiran-package-p (symbol-package (car second))))
         (car second))
        (t nil)))))

(defun walk-form-for-ichiran-syms (form set)
  "Collect every ichiran-package symbol from FORM into SET (a hash-set).
   No filtering against *universe* here — that's done on the Python side
   so the md output preserves all observations for transparency."
  (labels ((w (x)
             (cond
               ((and (symbolp x)
                     (symbol-package x)
                     (ichiran-package-p (symbol-package x)))
                (setf (gethash x set) t))
               ((consp x) (w (car x)) (w (cdr x))))))
    (w form)))

(defun read-and-walk-file (path)
  "Read every top-level form from PATH; populate *source-walk-deps*.
   Tracks (in-package ...) forms so unqualified symbols read into the
   correct package — same as how LOAD would intern them."
  (handler-case
      (with-open-file (s path :direction :input :external-format :utf-8)
        (let ((*package* (find-package :common-lisp-user))
              (*read-eval* nil))
          (loop for form = (handler-case (read s nil :seof)
                             (error (c)
                               (format t ";;   read error in ~a: ~a~%" path c)
                               :seof))
                until (eq form :seof)
                do (cond
                     ((and (consp form)
                           (symbolp (car form))
                           (string= (symbol-name (car form)) "IN-PACKAGE")
                           (consp (cdr form)))
                      (let* ((arg (cadr form))
                             (name (cond ((symbolp arg) (symbol-name arg))
                                         ((stringp arg) arg)
                                         (t arg)))
                             (pkg (handler-case (find-package name)
                                    (error () nil))))
                        (when pkg (setf *package* pkg))))
                     (t
                      (let ((defsym (defining-symbol-of-form form)))
                        (when defsym
                          (let ((set (or (gethash defsym *source-walk-deps*)
                                         (setf (gethash defsym *source-walk-deps*)
                                               (make-hash-table :test 'eq)))))
                            (walk-form-for-ichiran-syms form set)))))))))
    (error (c)
      (format t ";; error walking ~a: ~a~%" path c))))

(defun discover-source-files ()
  "Distinct .lisp pathnames mentioned by any universe symbol's source info.
   Iterates over a few definition-source kinds because some files only
   define classes or globals and would be missed by :function alone."
  (let ((seen (make-hash-table :test 'equal))
        (out nil))
    (loop for sym being the hash-keys of *universe* do
      (dolist (kind '(:function :variable :class :structure :macro))
        (handler-case
            (dolist (def (sb-introspect:find-definition-sources-by-name sym kind))
              (let ((path (sb-introspect:definition-source-pathname def)))
                (when (and path
                           (let ((tp (pathname-type path)))
                             (and tp (string-equal tp "lisp")))
                           (not (gethash (namestring path) seen)))
                  (setf (gethash (namestring path) seen) t)
                  (push path out))))
          (error () nil))))
    out))

(defun run-source-walk ()
  (let ((files (discover-source-files)))
    (format t ";; Source-walk: ~a files...~%" (length files))
    (dolist (path files)
      (read-and-walk-file path))
    (format t ";; Source-walk: ~a defining symbols recorded.~%"
            (hash-table-count *source-walk-deps*))))

(defun emit-source-walk-section (out sym)
  "Append the '## Source-walked references' block to OUT for SYM."
  (let ((set (gethash sym *source-walk-deps*)))
    (format out "~%## Source-walked references~%~%")
    (cond
      ((or (null set) (zerop (hash-table-count set)))
       (format out "_(none detected)_~%"))
      (t
       (let ((sorted (sort
                      (loop for k being the hash-keys of set
                            unless (eq k sym)
                            collect k)
                      #'string< :key #'fmt-sym)))
         (cond
           ((null sorted)
            (format out "_(none detected)_~%"))
           (t
            (dolist (s sorted) (format out "- `~a`~%" (fmt-sym s))))))))))

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
          (format out "_(none detected)_~%"))
      (emit-source-walk-section out sym))))

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
              (format out "~%"))))
      (emit-source-walk-section out sym))))

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

;;; --- Class / struct / DAO introspection -----------------------------------
;;;
;;; The function pass above misses defstruct and defclass forms. This pass
;;; walks find-class for every symbol in the ichiran universe and emits one
;;; markdown file per class or struct. Files use suffixes (_class, _struct,
;;; _dao) so they cannot collide with same-named function md files.
;;;
;;; DAO classes (postmodern :metaclass dao-class) get extra sections for
;;; table name, primary key, and per-column type info.

(defstruct class-entry sym cls kind src filename basename)

(defun dao-class-p (cls)
  "Test by metaclass name string so we don't depend on postmodern being aliased."
  (string= "DAO-CLASS" (symbol-name (class-name (class-of cls)))))

(defun class-kind (cls)
  (cond
    ((typep cls 'structure-class) :struct)
    ((dao-class-p cls) :dao)
    (t :class)))

(defun safe-class-source (sym kind)
  (handler-case
      (first (sb-introspect:find-definition-sources-by-name
              sym (case kind (:struct :structure) (otherwise :class))))
    (error () nil)))

(defun collect-class-entries ()
  (let (out)
    (loop for sym being the hash-keys of *universe* do
      (let ((cls (handler-case (find-class sym nil) (error () nil))))
        (when cls
          (let ((spkg (and (class-name cls) (symbol-package (class-name cls)))))
            (when (ichiran-package-p spkg)
              (handler-case (sb-mop:finalize-inheritance cls) (error () nil))
              (let* ((kind (class-kind cls))
                     (src (safe-class-source sym kind)))
                (when src
                  (push (make-class-entry
                         :sym sym :cls cls :kind kind :src src
                         :filename (src-path src)
                         :basename (source-basename (src-path src)))
                        out))))))))
    out))

(defun dao-slot-col-info (slot)
  "Returns (col-type sql-name primary-key) using slot-value with bound checks."
  (let ((ct (find-symbol "COL-TYPE" :postmodern))
        (sn (find-symbol "SQL-NAME" :postmodern))
        (pk (find-symbol "COL-PRIMARY-KEY" :postmodern)))
    (list
     (and ct (slot-boundp slot ct) (slot-value slot ct))
     (and sn (slot-boundp slot sn) (slot-value slot sn))
     (and pk (slot-boundp slot pk) (slot-value slot pk)))))

(defun fmt-name-list (names)
  "Format a list of symbol-ish names as comma-separated backtick-wrapped lower."
  (if names
      (format nil "~{`~(~a~)`~^, ~}" names)
      "_(none)_"))

(defun write-dao-md (out cls)
  (let ((tn (find-symbol "DAO-TABLE-NAME" :postmodern))
        (kk (find-symbol "DAO-KEYS" :postmodern))
        (cs (find-symbol "DAO-COLUMN-SLOTS" :postmodern)))
    (format out "**Table:** `~a`  ~%"
            (and tn (handler-case (funcall tn cls) (error () "?"))))
    (format out "**Primary key:** `~a`~%~%"
            (and kk (handler-case (funcall kk cls) (error () "?"))))
    (format out "## Inheritance~%~%")
    (format out "- Direct supers: ~a~%"
            (fmt-name-list (mapcar #'class-name (sb-mop:class-direct-superclasses cls))))
    (format out "- Precedence list: ~a~%~%"
            (fmt-name-list (mapcar #'class-name (sb-mop:class-precedence-list cls))))
    (format out "## Columns~%~%")
    (format out "| name | column | type | initargs | readers |~%")
    (format out "|---|---|---|---|---|~%")
    (when cs
      (dolist (slot (handler-case (funcall cs cls) (error () nil)))
        (let ((info (dao-slot-col-info slot)))
          (format out "| ~a | ~a | `~(~s~)` | ~a | ~a |~%"
                  (sb-mop:slot-definition-name slot)
                  (or (second info) "")
                  (or (first info) t)
                  (handler-case (sb-mop:slot-definition-initargs slot) (error () nil))
                  (handler-case (sb-mop:slot-definition-readers slot) (error () nil))))))
    (format out "~%")))

(defun write-struct-md (out sym)
  (let ((dd (handler-case (sb-kernel:find-defstruct-description sym) (error () nil))))
    (cond
      ((null dd)
       (format out "_(unable to retrieve defstruct description)_~%"))
      (t
       (format out "**Conc-name:** `~a`  ~%" (sb-kernel::dd-conc-name dd))
       (format out "**Default constructor:** `~a`  ~%" (sb-kernel::dd-default-constructor dd))
       (format out "**All constructors:** `~(~s~)`  ~%" (sb-kernel::dd-constructors dd))
       (format out "**Predicate:** `~a`  ~%" (sb-kernel::dd-predicate-name dd))
       (format out "**Copier:** `~a`  ~%" (sb-kernel::dd-copier-name dd))
       (format out "**Include:** `~a`  ~%" (sb-kernel::dd-include dd))
       (when (sb-kernel::dd-doc dd)
         (format out "**Documentation:** ~a  ~%" (escape-md (sb-kernel::dd-doc dd))))
       (format out "~%## Slots~%~%")
       (format out "| name | default | type | accessor |~%")
       (format out "|---|---|---|---|~%")
       (dolist (s (sb-kernel::dd-slots dd))
         (format out "| ~a | `~(~s~)` | `~(~s~)` | `~(~a~)` |~%"
                 (sb-kernel::dsd-name s)
                 (sb-kernel::dsd-default s)
                 (sb-kernel::dsd-type s)
                 (sb-kernel::dsd-accessor-name s)))
       (format out "~%")))))

(defun write-clos-md (out cls)
  (format out "## Inheritance~%~%")
  (format out "- Direct supers: ~a~%"
          (fmt-name-list (mapcar #'class-name (sb-mop:class-direct-superclasses cls))))
  (format out "- Precedence list: ~a~%~%"
          (fmt-name-list (mapcar #'class-name (sb-mop:class-precedence-list cls))))
  (format out "## Direct slots~%~%")
  (format out "| name | initform | allocation | initargs | readers | writers |~%")
  (format out "|---|---|---|---|---|---|~%")
  (dolist (s (sb-mop:class-direct-slots cls))
    (format out "| ~a | `~(~s~)` | ~a | ~a | ~a | ~a |~%"
            (sb-mop:slot-definition-name s)
            (handler-case (sb-mop:slot-definition-initform s) (error () nil))
            (handler-case (sb-mop:slot-definition-allocation s) (error () :instance))
            (handler-case (sb-mop:slot-definition-initargs s) (error () nil))
            (handler-case (sb-mop:slot-definition-readers s) (error () nil))
            (handler-case (sb-mop:slot-definition-writers s) (error () nil))))
  (format out "~%"))

(defun write-class-md (path entry)
  (ensure-directories-exist path)
  (let* ((sym (class-entry-sym entry))
         (cls (class-entry-cls entry))
         (kind (class-entry-kind entry))
         (src (class-entry-src entry))
         (file (source-basename (src-path src)))
         (line (or (offset-to-line (src-path src) (src-offset src))
                   (source-line-by-name
                    (src-path src)
                    (symbol-name sym)
                    (case kind
                      (:struct '("defstruct"))
                      (otherwise '("defclass")))))))
    (with-open-file (out path :direction :output
                              :if-exists :supersede :if-does-not-exist :create
                              :external-format :utf-8)
      (format out "# ~a (~a)~%~%"
              (string-downcase (symbol-name sym))
              (case kind (:struct "defstruct") (:dao "dao-class") (:class "defclass")))
      (format out "**Package:** `~a`  ~%" (pkg-prefix sym))
      (when file
        (if line
            (format out "**Source:** `~a:~a`  ~%" file line)
            (format out "**Source:** `~a`  ~%" file)))
      (format out "**Metaclass:** `~(~a~)`~%~%" (class-name (class-of cls)))
      (case kind
        (:struct (write-struct-md out sym))
        (:dao    (write-dao-md out cls))
        (:class  (write-clos-md out cls))))))

(defun class-suffix (kind)
  (case kind (:struct "_struct") (:dao "_dao") (:class "_class")))

(defun emit-class-entries (entries)
  (let ((groups (make-hash-table :test 'equal)))
    (dolist (e entries)
      (push e (gethash (class-entry-basename e) groups)))
    (loop for base being the hash-keys of groups
          for es = (gethash base groups) do
      (dolist (e es)
        (let* ((sym (class-entry-sym e))
               (fname (concatenate 'string
                                   (sanitize-name (string-downcase (symbol-name sym)))
                                   (class-suffix (class-entry-kind e))
                                   ".md"))
               (out-path (native-path
                          (concatenate 'string *output-dir-string* base "/" fname))))
          (handler-case
              (write-class-md out-path e)
            (error (c)
              (format t ";; SKIP class ~a (~a): ~a~%"
                      sym (class-entry-kind e) c))))))
    groups))

(defun extend-index (fn-groups cl-groups)
  "Append a Classes/Structs section per source file into the existing index.md."
  (let ((index (native-path (concatenate 'string *output-dir-string* "index.md"))))
    (with-open-file (out index :direction :output
                               :if-exists :append :if-does-not-exist :create
                               :external-format :utf-8)
      (declare (ignore fn-groups))
      (format out "~%---~%~%# Classes / structs / DAO~%~%")
      (let ((bases (sort (loop for k being the hash-keys of cl-groups collect k) #'string<)))
        (dolist (base bases)
          (let* ((es (sort (copy-list (gethash base cl-groups))
                           #'string<
                           :key (lambda (e) (string-downcase (symbol-name (class-entry-sym e)))))))
            (format out "## ~a~%~%" base)
            (dolist (e es)
              (let* ((sym (class-entry-sym e))
                     (fname (concatenate 'string
                                         (sanitize-name (string-downcase (symbol-name sym)))
                                         (class-suffix (class-entry-kind e))
                                         ".md"))
                     (kindlbl (case (class-entry-kind e)
                                (:struct "struct") (:dao "dao-class") (:class "class"))))
                (format out "- [~a](~a/~a) — ~a (`~a`)~%"
                        (string-downcase (symbol-name sym))
                        base fname kindlbl (pkg-prefix sym))))
            (format out "~%")))))))

;;; --- Global value introspection ------------------------------------------
;;;
;;; Walks every bound (value-cell) symbol in the ichiran universe and emits
;;; one md per global. Captures source location, kind (constant vs variable),
;;; type of value, and the printed value itself (best-effort: tries readable
;;; print first, falls back to a structural dump for hash tables and other
;;; non-readable shapes).

(defstruct global-entry sym src filename basename kind)

(defun global-kind (sym)
  (if (constantp sym) :constant :variable))

(defun safe-global-source (sym)
  (handler-case
      (first (sb-introspect:find-definition-sources-by-name sym :variable))
    (error () nil)))

(defun collect-global-entries ()
  (let (out)
    (loop for sym being the hash-keys of *universe* do
      (when (and (boundp sym)
                 (not (handler-case (find-class sym nil) (error () nil))))
        (let ((src (safe-global-source sym)))
          (when src
            (push (make-global-entry
                   :sym sym :src src
                   :filename (src-path src)
                   :basename (source-basename (src-path src))
                   :kind (global-kind sym))
                  out)))))
    out))

(defparameter *global-value-max-chars* 50000
  "Cap on the printed length of a single global's value. Bigger values get
   truncated with a marker. Stops one bad global from producing a multi-MB md.")

(defparameter *global-value-print-length* 1000)
(defparameter *global-value-print-level* 20)

(defun safe-prin1 (value)
  "Try a readable prin1; fall back to non-readable. Returns
   (text truncated-p readable-p)."
  (labels ((with-caps (readably)
             (let ((*print-readably* readably)
                   (*print-circle* t)
                   (*print-length* *global-value-print-length*)
                   (*print-level* *global-value-print-level*)
                   (*print-pretty* nil))
               (prin1-to-string value)))
           (cap (text readable)
             (if (> (length text) *global-value-max-chars*)
                 (values (subseq text 0 *global-value-max-chars*) t readable)
                 (values text nil readable))))
    (handler-case (cap (with-caps t) t)
      (error ()
        (handler-case (cap (with-caps nil) nil)
          (error (c)
            (values (format nil "<unprintable: ~a>" c) nil nil)))))))

(defun hash-table-dump (ht)
  "Structural dump of a hash table — keys and values, capped."
  (let ((*print-readably* nil)
        (*print-circle* t)
        (*print-length* 200)
        (*print-level* 10)
        (*print-pretty* nil)
        (n 0)
        (cap *global-value-print-length*))
    (with-output-to-string (out)
      (format out "(:hash-table :test ~(~a~) :count ~a"
              (hash-table-test ht) (hash-table-count ht))
      (block walking
        (maphash
         (lambda (k v)
           (when (>= n cap)
             (format out "~%  ;; ... ~a more entries elided"
                     (- (hash-table-count ht) cap))
             (return-from walking))
           (handler-case
               (format out "~%  (~s . ~s)" k v)
             (error (c)
               (format out "~%  ;; <unprintable entry: ~a>" c)))
           (incf n))
         ht))
      (format out ")"))))

(defun write-global-md (path entry)
  (ensure-directories-exist path)
  (let* ((sym (global-entry-sym entry))
         (src (global-entry-src entry))
         (kind (global-entry-kind entry))
         (val (handler-case (symbol-value sym) (error () :unbound)))
         (file (source-basename (src-path src)))
         (line (or (offset-to-line (src-path src) (src-offset src))
                   (source-line-by-name
                    (src-path src) (symbol-name sym)
                    '("defparameter" "defvar" "defconstant")))))
    (with-open-file (out path :direction :output
                              :if-exists :supersede :if-does-not-exist :create
                              :external-format :utf-8)
      (format out "# ~a (~a)~%~%"
              (string-downcase (symbol-name sym))
              (case kind (:constant "global constant") (otherwise "global variable")))
      (format out "**Package:** `~a`  ~%" (pkg-prefix sym))
      (when file
        (if line
            (format out "**Source:** `~a:~a`  ~%" file line)
            (format out "**Source:** `~a`  ~%" file)))
      (format out "**Type of value:** `~(~a~)`~%~%" (type-of val))
      (let ((doc (handler-case (documentation sym 'variable) (error () nil))))
        (when doc (format out "**Documentation:** ~a~%~%" (escape-md doc))))
      (format out "## Value~%~%")
      (cond
        ((eq val :unbound)
         (format out "_(symbol-value raised an error — value cell may have been cleared)_~%"))
        ((hash-table-p val)
         (format out "Hash-table — ~a entries, test: `~(~a~)`~%~%```lisp~%~a~%```~%"
                 (hash-table-count val) (hash-table-test val)
                 (hash-table-dump val)))
        (t
         (multiple-value-bind (text trunc readable) (safe-prin1 val)
           (format out "```lisp~%~a~%```~%" text)
           (cond
             ((and trunc (not readable))
              (format out "~%_(value truncated and is not round-trippable via `read`)_~%"))
             (trunc
              (format out "~%_(value truncated at ~a chars)_~%" *global-value-max-chars*))
             ((not readable)
              (format out "~%_(value is not round-trippable via `read` — likely contains closures, classes, or other unreadable shapes)_~%")))))))))

(defun emit-global-entries (entries)
  (let ((groups (make-hash-table :test 'equal)))
    (dolist (e entries)
      (push e (gethash (global-entry-basename e) groups)))
    (loop for base being the hash-keys of groups
          for es = (gethash base groups) do
      (dolist (e es)
        (let* ((sym (global-entry-sym e))
               (fname (concatenate 'string
                                   (sanitize-name (string-downcase (symbol-name sym)))
                                   "_global.md"))
               (out-path (native-path
                          (concatenate 'string *output-dir-string* base "/" fname))))
          (handler-case
              (write-global-md out-path e)
            (error (c)
              (format t ";; SKIP global ~a: ~a~%" sym c))))))
    groups))

(defun extend-index-globals (gl-groups)
  (let ((index (native-path (concatenate 'string *output-dir-string* "index.md"))))
    (with-open-file (out index :direction :output
                               :if-exists :append :if-does-not-exist :create
                               :external-format :utf-8)
      (format out "~%---~%~%# Globals (defparameter / defvar / defconstant)~%~%")
      (let ((bases (sort (loop for k being the hash-keys of gl-groups collect k) #'string<)))
        (dolist (base bases)
          (let* ((es (sort (copy-list (gethash base gl-groups))
                           #'string<
                           :key (lambda (e) (string-downcase (symbol-name (global-entry-sym e)))))))
            (format out "## ~a~%~%" base)
            (dolist (e es)
              (let* ((sym (global-entry-sym e))
                     (fname (concatenate 'string
                                         (sanitize-name (string-downcase (symbol-name sym)))
                                         "_global.md"))
                     (kindlbl (case (global-entry-kind e)
                                (:constant "const") (otherwise "var"))))
                (format out "- [~a](~a/~a) — ~a (`~a`)~%"
                        (string-downcase (symbol-name sym))
                        base fname kindlbl (pkg-prefix sym))))
            (format out "~%")))))))

;;; --- Run -------------------------------------------------------------------
(format t ";; Building call graph (who-calls inversion)...~%")
(setf *call-graph* (build-call-graph))
(format t ";; Call graph entries: ~a~%" (hash-table-count *call-graph*))

(format t ";; Running source-walk pass...~%")
(run-source-walk)

(format t ";; Collecting function entries...~%")
(let* ((entries (collect-entries))
       (class-entries (collect-class-entries))
       (global-entries (collect-global-entries)))
  (format t ";; ~a function entries, ~a class/struct entries, ~a globals.~%"
          (length entries) (length class-entries) (length global-entries))
  (let ((fn-groups (emit-all entries))
        (cl-groups (emit-class-entries class-entries))
        (gl-groups (emit-global-entries global-entries)))
    (write-index fn-groups)
    (extend-index fn-groups cl-groups)
    (extend-index-globals gl-groups)
    (format t ";; Wrote ~a function dirs, ~a class dirs, ~a global dirs.~%"
            (hash-table-count fn-groups)
            (hash-table-count cl-groups)
            (hash-table-count gl-groups))))

(format t ";; Done. Output: ~a~%" *output-dir*)
