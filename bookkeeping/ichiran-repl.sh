#!/bin/bash
# Ichiran REPL wrapper — strips SBCL banner, warnings, prompts, empty lines
# Usage:
#   ./ichiran-repl.sh <<'LISP'
#   (ichi:score "怖")
#   LISP
#
# Preloads shorthand package (ichi) with common helpers:
#   (ichi:score TEXT)        — calc-score for first find-word result
#   (ichi:words TEXT)        — all find-word results: seq, common, score, conj
#   (ichi:trace TEXT)        — detailed scoring trace: conj-data, types, inheritance
#   (ichi:entry SEQ)         — entry info: root-p, n-kanji, kanji/kana forms
#   (ichi:split TEXT)        — get-split result
#   (ichi:conj SEQ)          — conjugation rows + CSR
#   (ichi:uk SEQ)            — sense-prop uk check
#   (ichi:posi SEQS)         — get-non-arch-posi for seq list
#   (ichi:lmc LEN MODE)      — length-multiplier-coeff

HELPERS='
(defpackage :ichi (:use :cl)
  (:export :score :words :trace-word :entry-info :split-info :uk :posi :lmc :conj-rows :full-trace))
(in-package :ichi)

(defun score (text)
  (ichiran/conn:with-db nil
    (let ((w (car (ichiran/dict::find-word text))))
      (when w (format t "~a~%" (ichiran/dict::calc-score w))))))

(defun words (text)
  (ichiran/conn:with-db nil
    (dolist (w (ichiran/dict::find-word text))
      (format t "seq=~a common=~a score=~a conj=~a type=~a~%"
        (slot-value w (quote ichiran/dict::seq))
        (slot-value w (quote ichiran/dict::common))
        (ichiran/dict::calc-score w)
        (ichiran/dict::word-conjugations w)
        (type-of w)))))

(defun trace-word (text)
  (ichiran/conn:with-db nil
    (let* ((w (car (ichiran/dict::find-word text)))
           (seq (slot-value w (quote ichiran/dict::seq)))
           (cd (ichiran/dict::word-conj-data w)))
      (format t "seq=~a type=~a common=~a conj=~a score=~a~%"
        seq (type-of w) (slot-value w (quote ichiran/dict::common))
        (ichiran/dict::word-conjugations w) (ichiran/dict::calc-score w))
      (when cd
        (let ((filtered cd))
          (unless (every (function ichiran/dict::conj-data-via) cd)
            (setf filtered (remove-if (function ichiran/dict::conj-data-via) cd)))
          (let* ((props (mapcar (function ichiran/dict::conj-data-prop) filtered))
                 (types (mapcar (function ichiran/dict::conj-type) props)))
            (format t "conj-types=~a weak=~a~%" types
              (every (lambda (p) (ichiran/dict::test-conj-prop p ichiran/dict::*weak-conj-forms*)) props)))
          (dolist (c filtered)
            (format t "  from=~a via=~a~%" (ichiran/dict::conj-data-from c) (ichiran/dict::conj-data-via c)))))
      ;; inheritance
      (when cd
        (let ((ot (ichiran/dict::get-original-text w :conj-data
                    (if (every (function ichiran/dict::conj-data-via) cd) cd
                        (remove-if (function ichiran/dict::conj-data-via) cd)))))
          (when ot
            (format t "orig: ~{~a~^, ~}~%"
              (mapcar (lambda (o) (format nil "~a common=~a ord=~a"
                (ichiran/dict::text o) (slot-value o (quote ichiran/dict::common)) (ichiran/dict::ord o))) ot)))))
      ;; split
      (when cd
        (let ((conj-of (mapcar (function ichiran/dict::conj-data-from)
                (if (every (function ichiran/dict::conj-data-via) cd) cd
                    (remove-if (function ichiran/dict::conj-data-via) cd)))))
          (multiple-value-bind (sp sm) (ichiran/dict::get-split w conj-of)
            (when sp (format t "split=~a smod=~a~%" sp sm)))))
      ;; entry info
      (let ((e (postmodern:get-dao (quote ichiran/dict::entry) seq)))
        (format t "root-p=~a n-kanji=~a~%" (ichiran/dict::root-p e) (ichiran/dict::n-kanji e)))
      ;; sp-seq-set and prefer-kana
      (let* ((e (postmodern:get-dao (quote ichiran/dict::entry) seq))
             (wc (ichiran/dict::word-conjugations w))
             (conj-only (and wc (not (eql wc :root))))
             (root-p (and (not conj-only) (ichiran/dict::root-p e)))
             (sp (if root-p (list seq)
                     (cons seq (when cd (mapcar (function ichiran/dict::conj-data-from)
                       (if (every (function ichiran/dict::conj-data-via) cd) cd
                           (remove-if (function ichiran/dict::conj-data-via) cd))))))))
        (let ((pk (postmodern:select-dao (quote ichiran/dict::sense-prop)
                    (:and (:in (quote seq) (:set sp)) (:= (quote tag) "misc") (:= (quote text) "uk")))))
          (format t "prefer-kana=~a posi=~a~%" (not (null pk))
            (ichiran/dict::get-non-arch-posi
              (cons seq (when cd (mapcar (function ichiran/dict::conj-data-from)
                (if (every (function ichiran/dict::conj-data-via) cd) cd
                    (remove-if (function ichiran/dict::conj-data-via) cd))))))))))))

(defun entry-info (seq)
  (ichiran/conn:with-db nil
    (let ((e (postmodern:get-dao (quote ichiran/dict::entry) seq)))
      (format t "root-p=~a n-kanji=~a pn=~a~%" (ichiran/dict::root-p e) (ichiran/dict::n-kanji e) (ichiran/dict::primary-nokanji e)))
    (let ((kt (postmodern:select-dao (quote ichiran/dict::kanji-text) (:= (quote seq) seq))))
      (dolist (k kt) (format t "kanji: ~a common=~a ord=~a~%" (ichiran/dict::text k) (slot-value k (quote ichiran/dict::common)) (ichiran/dict::ord k))))
    (let ((kn (postmodern:select-dao (quote ichiran/dict::kana-text) (:= (quote seq) seq))))
      (dolist (k kn) (format t "kana: ~a common=~a ord=~a~%" (ichiran/dict::text k) (slot-value k (quote ichiran/dict::common)) (ichiran/dict::ord k))))))

(defun split-info (text)
  (ichiran/conn:with-db nil
    (let* ((w (car (ichiran/dict::find-word text)))
           (cd (ichiran/dict::word-conj-data w))
           (conj-of (when cd (mapcar (function ichiran/dict::conj-data-from)
             (if (every (function ichiran/dict::conj-data-via) cd) cd
                 (remove-if (function ichiran/dict::conj-data-via) cd))))))
      (multiple-value-bind (sp sm) (ichiran/dict::get-split w conj-of)
        (format t "split=~a~%smod=~a~%" sp sm)))))

(defun uk (seq)
  (ichiran/conn:with-db nil
    (let ((props (postmodern:select-dao (quote ichiran/dict::sense-prop)
                   (:and (:= (quote seq) seq) (:= (quote tag) "misc") (:= (quote text) "uk")))))
      (format t "uk=~a~%" (not (null props))))))

(defun posi (seqs)
  (ichiran/conn:with-db nil
    (format t "~a~%" (ichiran/dict::get-non-arch-posi seqs))))

(defun lmc (len mode)
  (format t "~a~%" (ichiran/dict::length-multiplier-coeff len mode)))

(defun conj-rows (seq)
  (ichiran/conn:with-db nil
    (let ((rows (postmodern:query
                  (format nil "SELECT c.id, c.\"from\", c.via, csr.text, csr.source_text FROM conjugation c JOIN conj_source_reading csr ON csr.conj_id = c.id WHERE c.seq = ~a" seq) :rows)))
      (dolist (r rows)
        (format t "from=~a via=~a text=~a src=~a~%" (second r) (third r) (fourth r) (fifth r))))))

(defun full-trace (text)
  "Compute all calc-score intermediates — matches TRACE_SEQ output format"
  (ichiran/conn:with-db nil
    (dolist (w (ichiran/dict::find-word text))
      (let* ((seq (slot-value w (quote ichiran/dict::seq)))
             (e (postmodern:get-dao (quote ichiran/dict::entry) seq))
             (wc (ichiran/dict::word-conjugations w))
             (cd (ichiran/dict::word-conj-data w))
             (conj-only (and wc (not (eql wc :root))))
             (common-init (if conj-only :null (slot-value w (quote ichiran/dict::common))))
             (common-p-init (not (eql common-init :null)))
             (ord-init (ichiran/dict::ord w))
             (kanji-p (eql (type-of w) (quote ichiran/dict::kanji-text)))
             (n-kanji (ichiran/dict::count-char-class text :kanji))
             (len (max 1 (ichiran/dict::mora-length text)))
             (root-p (and (not conj-only) (ichiran/dict::root-p e)))
             (common common-init) (common-p common-p-init) (ord ord-init))
        ;; inheritance
        (when (and cd (not (and (= ord 0) common-p)))
          (let ((filtered (if (every (function ichiran/dict::conj-data-via) cd) cd
                              (remove-if (function ichiran/dict::conj-data-via) cd))))
            (let ((ot (ichiran/dict::get-original-text w :conj-data filtered)))
              (when ot
                (unless common-p
                  (let ((cc (remove :null (mapcar (lambda (o) (slot-value o (quote ichiran/dict::common))) ot))))
                    (when cc (setf common 0 common-p t))))
                (let ((mo (reduce (function min) ot :key (function ichiran/dict::ord))))
                  (when (< mo ord) (setf ord mo)))))))
        ;; sp-seq-set
        (let* ((filtered-cd (when cd (if (every (function ichiran/dict::conj-data-via) cd) cd
                                        (remove-if (function ichiran/dict::conj-data-via) cd))))
               (conj-of (mapcar (function ichiran/dict::conj-data-from) filtered-cd))
               (seq-set (cons seq conj-of))
               (sp (if (and root-p (not nil)) (list seq) seq-set))
               (prefer-kana (not (null (postmodern:select-dao (quote ichiran/dict::sense-prop)
                 (:and (:in (quote seq) (:set sp)) (:= (quote tag) "misc") (:= (quote text) "uk"))))))
               (conj-types (mapcar (function ichiran/dict::conj-type) (mapcar (function ichiran/dict::conj-data-prop) filtered-cd)))
               (conj-types-p (or root-p nil (not (every (lambda (p) (ichiran/dict::test-conj-prop p ichiran/dict::*weak-conj-forms*))
                 (mapcar (function ichiran/dict::conj-data-prop) filtered-cd)))))
               (is-arch (every (function ichiran/dict::is-arch) sp))
               (secondary (and cd (every (function ichiran/dict::conj-data-via) cd)))
               (long-threshold (cond
                 ((and kanji-p (not prefer-kana) (or (and root-p (not cd)) (and nil (member 13 conj-types)))) 2)
                 ((and common-p (typep common (quote fixnum)) (< 0 common 10)) 2)
                 ((and (intersection (quote (3 9)) conj-types) (not nil)) 4)
                 (t 3)))
               (long-p (> len long-threshold))
               (primary-p (and (not is-arch) (or
                 (and prefer-kana conj-types-p (not kanji-p))
                 (and (or (= ord 0) nil) (or kanji-p conj-types-p)
                      (or (and kanji-p (not prefer-kana)) (= (ichiran/dict::n-kanji e) 0)))
                 (and prefer-kana kanji-p (= ord 0)))))
               (primary-bonus (if (not primary-p) 0
                 (cond (long-p 10) ((and secondary (not kanji-p)) 2) ((and common-p conj-types-p) 5) ((or prefer-kana (not kanji-p)) 3) (t 2))))
               (no-common-bonus (or nil (not conj-types-p)))
               (common-bonus (if (or (not common-p) no-common-bonus) 0
                 (cond
                   ((and secondary (not nil)) (if (and kanji-p primary-p) 4 2))
                   ((or long-p nil (and root-p (or kanji-p (and primary-p (> len 2)))))
                    (cond ((and (typep common (quote fixnum)) (= common 0)) 10)
                          ((not primary-p) (max (- 15 common) 10))
                          (t (max (- 20 common) 10))))
                   (kanji-p 8) (primary-p 4) ((or (> len 2) (and (typep common (quote fixnum)) (< 0 common 10))) 3) (t 2))))
               (score-raw (+ 1 primary-bonus common-bonus))
               (score-floors score-raw))
          (when long-p (setf score-floors (max len score-floors)))
          (when kanji-p
            (setf score-floors (max 5 score-floors))
            (when (and long-p (or (> n-kanji 1) (> len 4))) (incf score-floors 2)))
          (format t "~%text=~a seq=~a len=~a n_kanji=~a~%" text seq len n-kanji)
          (format t "kanji_p=~a prefer_kana=~a is_arch=~a~%" kanji-p prefer-kana is-arch)
          (format t "common_tier=~a(init=~a) common_p=~a(init=~a) ord=~a(init=~a)~%" common common-init common-p common-p-init ord ord-init)
          (format t "conj_only=~a root_p=~a secondary=~a has_conj=~a~%" conj-only root-p secondary (not (null cd)))
          (format t "conj_types=~a weak=~a conj_types_p=~a~%" conj-types (and conj-types (not conj-types-p)) conj-types-p)
          (format t "long_threshold=~a long_p=~a primary_p=~a~%" long-threshold long-p primary-p)
          (format t "score = 1 + primary(~a) + common(~a) = ~a~%" primary-bonus common-bonus score-raw)
          (format t "prop_score=~a (after floors)~%" score-floors)
          (let* ((coeff (ichiran/dict::length-multiplier-coeff len (if kanji-p :strong :weak)))
                 (mk (if (> n-kanji 1) (* (1- n-kanji) 5) 0)))
            (format t "coeff=~a mk_bonus=~a final_score=~a~%" coeff mk (* score-floors (+ coeff mk))))
          (format t "actual_calc_score=~a~%" (ichiran/dict::calc-score w)))))))

(in-package :cl-user)
(format t "~%---READY---~%")
'
# Combine helpers + user input, pipe to SBCL on .103 native install
# (migrated from docker fa843f3e5d78 → /home/david/storage/ichiran native install, 2026-04-22)
{
  echo "$HELPERS"
  cat
} | ssh david@192.168.1.103 'sbcl --noinform --load ~/quicklisp/setup.lisp --eval "(ql:quickload :ichiran :silent t)"' 2>&1 \
  | sed '1,/^---READY---$/d' \
  | grep -v '^\* *$' \
  | grep -v '^NIL$' \
  | sed 's/^\* //' \
  | sed '/^$/d'

