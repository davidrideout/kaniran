//! Port of `ichiran/dict:load-extras` (`dict-load.lisp:185`).
//!
//! Build-time pipeline that rehydrates everything downstream of the
//! raw JMdict load: conjugations, secondary conjugations, custom data,
//! errata, and a final `entry` row-count refresh. Runs after
//! [`super::load_jmdict`] or in place of it when re-applying the
//! extras after [`super::drop_extras`].
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_errata::add_errata;
use super::load_conjugations::load_conjugations;
use super::load_secondary_conjugations::load_secondary_conjugations;
use super::recalc_entry_stats_all::recalc_entry_stats_all;
use crate::conn::kani_context::KaniranContext;
use crate::custom::load_custom_data::{load_custom_data, LoadCustomDataError};

pub async fn load_extras(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    println!("Loading conjugations...");
    load_conjugations(ctx).await?;
    println!("Loading secondary conjugations...");
    load_secondary_conjugations(ctx, None).await?;
    println!("Loading custom data...");
    // dict-load.lisp:191 (ichiran/custom:load-custom-data nil t)
    load_custom_data(ctx, &[], true).await?;
    add_errata(ctx).await?;
    recalc_entry_stats_all(ctx).await?;
    sqlx::query("ANALYZE").execute(&ctx.pool).await?;
    Ok(())
}
