# Known discrepancies between kaniran and ichiran

kaniran is a transliteration of [ichiran](https://github.com/tshatrov/ichiran):
the goal is identical output, not a reinterpretation. The cases below are
the only places where kaniran's result can differ from a given run of
ichiran, and each one traces back to ichiran itself not being fully
deterministic — its dictionary queries leave some row orderings
unspecified, and it mutates some lookup state mid-session.

In every case the *meaning* of the analysis — the words, readings,
glosses, and romanization — is identical. Only an internal identifier or
a synonymous label can differ.

## Unordered query results

ichiran looks words up without a total ordering, so when several
dictionary rows are equally valid, the one that "wins" is simply
whichever the database returns first. A different database build can
return the same rows in a different order.

```lisp
;; a word's readings come from a UNION with no ORDER BY
(:union (:select 'kt.* :from 'kana-text :where (:= 'kt.seq seq))
        (:select 'kt.* :from 'kana-text
                 :left-join 'conjugation :on (:= 'conj.seq 'kt.seq)
                 :where (:= 'conj.from seq)))
```

**kaniran** queries its own database the same way. For a word with
interchangeable rows — for example いる's negative forms, or a substring
that matches several entries — kaniran may keep a different row than a
particular ichiran run. The reading and romanization are the same; only
the underlying dictionary sequence number differs.

## "Passive" vs "Potential"

A visible consequence of the above. For ichidan (る) verbs the passive
and potential forms are spelled identically (…られる), so the dictionary
holds two separate conjugation entries for the one surface form. Which
entry a spelling resolves to follows the row order described above.

```lisp
("type" (get-conj-description (conj-type obj))) ; => "Passive" or "Potential"
```

**kaniran** reports whichever of the two entries its database selects.
Both describe the same surface form (e.g. 居られる), so the word and its
gloss are unchanged — only the conjugation label may read "Passive" where
an ichiran run showed "Potential", or the reverse.

## Conjugation base-form display pick

Another visible consequence of unordered queries. When several writings
of one entry conjugate to the same surface form — 体にいい and 体によい
both negate to 体によくない — the conjugation block's "reading" line has
to pick one base form to display. ichiran takes the first row of an
unordered lookup:

```lisp
;; find-words-seqs: no ORDER BY; the caller keeps (car ...)
(select-dao 'kanji-text (:and (:in 'text (:set kanji-words))
                              (:in 'seq (:set seqs))))
```

**kaniran** queries the same way, so the winner follows each database
instance's physical history (insert order, in-place updates,
dump/restore). kaniran may display 体によい 【からだによい】 where a
given ichiran run shows 体にいい 【からだにいい】. Both are listed
writings of the same dictionary entry and the conjugated surface derives
from both, so the word, reading, gloss, and score are identical — only
which synonymous base form is named differs.

## Mid-session state in ichiran

When ichiran assembles a compound word, it writes the compound's
conjugation list back through a dictionary-cache object that it *shares*
by reference. A later word that reuses that cached row then sees the
narrowed list, so a word's reported conjugations can depend on what
ichiran happened to process earlier in the same session.

```lisp
(defmethod (setf word-conjugations) (value (word compound-text))
  (setf (word-conjugations (car (last (words word)))) value))
```

**kaniran** copies cache rows rather than sharing them, so this never
happens: a word always reports its full conjugation set, independent of
processing order. Where an ichiran session has narrowed a row, kaniran
lists the complete set. For 出てけ, kaniran lists いけ as both the
imperative of 行く and the potential continuative; a narrowed ichiran run
lists only one.

## Conjugated-entry writing order (`ord`)

When the loader builds a conjugated entry, it gives each writing an `ord`
(its position within the entry) by sorting the generated writings — but
the source readings come from a `UNION` with no `ORDER BY`, finished with
a non-stable sort. So when an entry has several writings of the same kind,
their `ord` is effectively whatever order the database returned.
Source-entry `ord` is unaffected: it follows JMdict XML order and is
deterministic.

```lisp
;; readings pulled by an ORDER BY-less UNION, then a non-stable sort
(sort readings (lex-compare #'<) :key #'cdddr)
```

**kaniran** assigns `ord` the same way. The six conjugated forms of
ございます each carry four writings (御座いません, 御座居ません, ご座いません,
厶いません, and so on); kaniran numbers them differently from a given
ichiran build. The writing, reading, gloss, and romanization are
identical — only the internal `ord` (which writing is "first") differs,
and no output path reads it.
