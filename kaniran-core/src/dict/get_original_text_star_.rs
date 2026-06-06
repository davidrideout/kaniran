//! Port of `ichiran/dict:get-original-text*` (`dict.lisp:380`).
//!
//! For each input [`ConjData`] and the surface `texts`, returns
//! `(text, seq)` pairs naming the pre-conjugation reading rows to look
//! up: matched `src_text` paired with the conj-data's `from` when `via`
//! is nil, else recursing through [`get_conj_data`] to peel one layer
//! of secondary conjugation when `via` is set.

use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data_struct::ConjData;
use crate::dict::get_conj_data::{get_conj_data, FromOrConjIds};

pub async fn get_original_text_star_(
    ctx: &KaniranContext,
    conj_datas: &[ConjData],
    texts: &[&str],
) -> Result<Vec<(String, i32)>, sqlx::Error> {
    let mut out: Vec<(String, i32)> = Vec::new();
    for conj_data in conj_datas {
        let src_text: Vec<&str> = conj_data
            .src_map
            .iter()
            .filter(|(txt, _)| texts.iter().any(|t| *t == txt))
            .map(|(_, src_txt)| src_txt.as_str())
            .collect();
        let from = conj_data
            .from
            .expect("conj-data emitted by get-conj-data always has a non-nil `from` slot");
        match conj_data.via {
            None => {
                // dict.lisp:390 ((mapcar (lambda (txt) (list txt (conj-data-from conj-data))) src-text))
                for txt in &src_text {
                    out.push(((*txt).to_string(), from));
                }
            }
            Some(via) => {
                // dict.lisp:391-392 (recursive get-conj-data + get-original-text*)
                let new_cd = get_conj_data(ctx, via, FromOrConjIds::From(from), &[]).await?;
                let inner =
                    Box::pin(get_original_text_star_(ctx, &new_cd, &src_text)).await?;
                out.extend(inner);
            }
        }
    }
    Ok(out)
}

