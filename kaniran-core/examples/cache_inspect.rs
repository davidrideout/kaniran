//! One-shot cache-cardinality inspector. Builds a `KaniranContext`
//! against the configured DB and prints the size of every populated
//! cache, plus a small sample, for cross-checking against the
//! upstream Lisp `ensure :no-conj-data` etc. values.

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::_star_special_counters_star_::special_counters;

#[tokio::main]
async fn main() {
    let ctx = match KaniranContext::from_env().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cache_inspect: {}", e);
            std::process::exit(1);
        }
    };

    let total_args: usize = ctx.counter_cache.values().map(|v| v.len()).sum();
    let mut top: Vec<(&String, usize)> =
        ctx.counter_cache.iter().map(|(k, v)| (k, v.len())).collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    println!("no-conj-data    count={}", ctx.no_conj_data.len());
    println!("is-arch         count={}", ctx.is_arch.len());
    println!("special-counters count={}", special_counters().len());
    println!(
        "counter-cache   keys={} total-args={}",
        ctx.counter_cache.len(),
        total_args
    );

    println!("----");
    println!("counter-cache top 20 by args-count:");
    for (k, v) in top.iter().take(20) {
        println!("  {} {}", v, k);
    }

    println!("----");
    println!("no-conj-data sample (first 20 seqs sorted):");
    let mut ncd: Vec<i32> = ctx.no_conj_data.iter().copied().collect();
    ncd.sort();
    for s in ncd.iter().take(20) {
        println!("  {}", s);
    }

    println!("----");
    println!("is-arch sample (first 20 seqs sorted):");
    let mut arch: Vec<i32> = ctx.is_arch.iter().copied().collect();
    arch.sort();
    for s in arch.iter().take(20) {
        println!("  {}", s);
    }
}
