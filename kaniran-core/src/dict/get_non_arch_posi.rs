//! Port of `ichiran/dict:get-non-arch-posi` (`dict.lisp:762`).
//!
//! Returns the distinct list of `pos`-tagged property values for senses
//! inside `seq_set` whose containing sense does NOT carry an `arch` /
//! `obsc` / `rare` misc tag (an anti-join via `sp2.id IS NULL`).

use crate::conn::kani_context::KaniranContext;

pub async fn get_non_arch_posi(
    ctx: &KaniranContext,
    seq_set: &[i32],
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT sp1.text \
         FROM sense_prop sp1 \
         LEFT JOIN sense_prop sp2 \
                ON sp1.sense_id = sp2.sense_id \
               AND sp2.tag = 'misc' \
               AND sp2.text IN ('arch', 'obsc', 'rare') \
         WHERE sp1.seq = ANY($1) \
           AND sp1.tag = 'pos' \
           AND sp2.id IS NULL",
    )
    .bind(seq_set)
    .fetch_all(&ctx.pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::kani_context::KaniranContext;

    // All assertions REPL-pinned against upstream ichiran. Each test
    // sorts the returned Vec before comparing because the upstream
    // Lisp `(:select … :distinct …)` does not impose an ORDER BY,
    // and Postgres is free to return distinct rows in any order.
    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[tokio::test]
    async fn taberu_single_seq() {
        // (get-non-arch-posi '(1357400)) → ("v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["v5m".to_string(), "vt".to_string()]
        );
    }

    #[tokio::test]
    async fn no_particle_seq() {
        // (get-non-arch-posi '(2089020)) → ("aux-v" "cop" "cop-da")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2089020]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn dummy_seq_1000220() {
        // (get-non-arch-posi '(1000220)) → ("adj-na")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1000220]).await.expect("query");
        assert_eq!(sorted(got), vec!["adj-na".to_string()]);
    }

    #[tokio::test]
    async fn hon_noun_seq() {
        // (get-non-arch-posi '(1522150)) → ("ctr" "n" "pref")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1522150]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["ctr".to_string(), "n".to_string(), "pref".to_string()]
        );
    }

    #[tokio::test]
    async fn counter_seq_1325880() {
        // (get-non-arch-posi '(1325880)) → ("n")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1325880]).await.expect("query");
        assert_eq!(sorted(got), vec!["n".to_string()]);
    }

    #[tokio::test]
    async fn two_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020))
        //   → ("aux-v" "cop" "cop-da" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn zo_particle_seq() {
        // (get-non-arch-posi '(2029110)) → ("int" "prt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[2029110]).await.expect("query");
        assert_eq!(
            sorted(got),
            vec!["int".to_string(), "prt".to_string()]
        );
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // (get-non-arch-posi '(99999999)) → NIL
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[99999999]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn empty_seq_set_returns_empty() {
        // (get-non-arch-posi nil) → NIL.
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[]).await.expect("query");
        assert!(got.is_empty(), "expected NIL, got {got:?}");
    }

    #[tokio::test]
    async fn many_seqs_union() {
        // (get-non-arch-posi '(1357400 2089020 1522150 1000220))
        //   → ("adj-na" "aux-v" "cop" "cop-da" "ctr" "n" "pref" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2089020, 1522150, 1000220])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec![
                "adj-na".to_string(),
                "aux-v".to_string(),
                "cop".to_string(),
                "cop-da".to_string(),
                "ctr".to_string(),
                "n".to_string(),
                "pref".to_string(),
                "v5m".to_string(),
                "vt".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn taberu_with_conj_root() {
        // (get-non-arch-posi (list 1357400 2027820)) → ("exp" "v5m" "vt")
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = get_non_arch_posi(&ctx, &[1357400, 2027820])
            .await
            .expect("query");
        assert_eq!(
            sorted(got),
            vec!["exp".to_string(), "v5m".to_string(), "vt".to_string()]
        );
    }
}
