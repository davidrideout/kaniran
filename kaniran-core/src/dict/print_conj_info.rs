//! Port of `ichiran/dict:print-conj-info` (`dict.lisp:1648`).
//!
//! ```lisp
//! (defun print-conj-info (seq &key conjugations (out *standard-output*))
//!   (loop with via-used = nil
//!      for (conj props) in (select-conjs-and-props seq conjugations)
//!      for via = (seq-via conj)
//!      unless (member via via-used)
//!      do (loop for conj-prop in props
//!            for first = t then nil
//!            do (format out "~%~:[ ~;[~] Conjugation: ~a" first (conj-info-short conj-prop)))
//!        (if (eql via :null)
//!            (format out "~%  ~a" (entry-info-short (seq-from conj)))
//!            (progn
//!              (format out "~% --(via)--")
//!              (print-conj-info via :out out)
//!              (push via via-used)))
//!        (princ " ]" out)))
//! ```
//!
//! Diverges by taking `&KaniranContext` for the DB handle (upstream
//! `*connection*`). The `:out` keyword (default `*standard-output*`)
//! becomes a required `&mut String`; every upstream caller binds a
//! string-output-stream. `conjugations` is `Option<&WordConjugations>`,
//! threaded to `select-conjs-and-props` with nil text.

use crate::conn::kani_context::KaniranContext;

use super::conj_info_short::conj_info_short;
use super::entry_info_short::entry_info_short;
use super::filter_props::FilterPropsText;
use super::select_conjs_and_props::select_conjs_and_props;
use super::simple_text_class::WordConjugations;

pub async fn print_conj_info(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: Option<&WordConjugations>,
    out: &mut String,
) -> Result<(), sqlx::Error> {
    let mut via_used: Vec<Option<i32>> = Vec::new();
    // (select-conjs-and-props seq conjugations) — print-conj-info passes no text
    for (conj, props, _) in
        select_conjs_and_props(ctx, seq, conjugations, FilterPropsText::None).await?
    {
        let via = conj.seq_via;
        // unless (member via via-used)
        if via_used.contains(&via) {
            continue;
        }
        // dict.lisp:1655 — "~%~:[ ~;[~] Conjugation: ~a" (first → "[", else " ")
        let mut first = true;
        for conj_prop in &props {
            out.push_str(&format!(
                "\n{} Conjugation: {}",
                if first { "[" } else { " " },
                conj_info_short(conj_prop)
            ));
            first = false;
        }
        // (if (eql via :null) ...)
        match via {
            None => {
                // (format out "~%  ~a" (entry-info-short (seq-from conj)))
                out.push_str(&format!(
                    "\n  {}",
                    entry_info_short(ctx, conj.seq_from, None).await?
                ));
            }
            Some(via_seq) => {
                // (format out "~% --(via)--")
                out.push_str("\n --(via)--");
                // (print-conj-info via :out out)
                Box::pin(print_conj_info(ctx, via_seq, None, out)).await?;
                // (push via via-used)
                via_used.push(via);
            }
        }
        // (princ " ]" out)
        out.push_str(" ]");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn render(
        ctx: &KaniranContext,
        seq: i32,
        conjugations: Option<&WordConjugations>,
    ) -> String {
        let mut out = String::new();
        print_conj_info(ctx, seq, conjugations, &mut out)
            .await
            .unwrap();
        out
    }

    /// REPL fixtures (.103, `ichiran/dict::print-conj-info` via
    /// `(with-output-to-string (s) (print-conj-info seq :out s))`),
    /// 2026-05-24, `conjugations` = nil. Covers:
    /// - 1156880: two via-null conjugations, one prop each — the
    ///   entry-info-short branch repeated, each prop opens with "[".
    /// - 1184270: one via-null conjugation with two props — the
    ///   `first` toggle ("[" then " ") inside one " ]".
    /// - 1257260: two non-null via conjugations — the " --(via)--"
    ///   branch with a recursive call producing the via entry.
    /// - 10674648: two conjugations sharing via 10327845 — the second
    ///   is dropped by `(member via via-used)`, so only one block prints.
    /// - 1358280: no conjugations (root) → empty output.
    #[tokio::test]
    async fn print_conj_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (
                1156880,
                "\n[ Conjugation: [v1] Continuative (~i)\n  慰める 【なぐさめる】 : to comfort; to console; to amuse ]\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            ),
            (
                1184270,
                "\n[ Conjugation: [v5aru] Imperative Affirmative Plain\n  Conjugation: [v5aru] Continuative (~i)\n  下さる 【くださる】 : to give; to confer; to bestow ]",
            ),
            (
                1257260,
                "\n[ Conjugation: [v1] Continuative (~i)\n --(via)--\n[ Conjugation: [v5r] Causative Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]\n[ Conjugation: [v5s] Imperative Affirmative Plain\n --(via)--\n[ Conjugation: [v5r] Causative (~su) Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]",
            ),
            (
                10674648,
                "\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n --(via)--\n[ Conjugation: [v5s] Potential Affirmative Plain\n  くねらす : to wriggle; to twist (one's body); to writhe ]\n[ Conjugation: [v5r] Causative Affirmative Plain\n  くねる : to bend loosely back and forth; to wriggle; to be crooked ] ]",
            ),
            (1358280, ""),
        ];
        for (seq, expected) in cases {
            assert_eq!(&render(&ctx, *seq, None).await, expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103, `print-conj-info 1156880` with
    /// `:conjugations`), 2026-05-24. `:root` selects no conjugations
    /// (empty output); an explicit id list narrows to that single
    /// conjugation.
    #[tokio::test]
    async fn print_conj_info_conjugations_arg() {
        let ctx = ctx_from_env().await;
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Root)).await,
            "",
            "conjugations=:root"
        );
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552]))).await,
            "\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            "conjugations=(366552)"
        );
    }
}
