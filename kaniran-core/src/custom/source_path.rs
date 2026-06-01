//! Port of `ichiran/custom:source-path` (`dict-custom.lisp:315`).

use std::path::PathBuf;

pub fn source_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("sources")
        .join(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_path_joins_data_sources() {
        let p = source_path("extra.xml");
        assert!(p.is_absolute(), "expected absolute path, got {p:?}");
        assert!(p.ends_with("data/sources/extra.xml"), "got {p:?}");
        let p = source_path("jichitai.csv");
        assert!(p.is_absolute(), "expected absolute path, got {p:?}");
        assert!(p.ends_with("data/sources/jichitai.csv"), "got {p:?}");
        let p = source_path("gyoseiku.csv");
        assert!(p.is_absolute(), "expected absolute path, got {p:?}");
        assert!(p.ends_with("data/sources/gyoseiku.csv"), "got {p:?}");
    }
}
