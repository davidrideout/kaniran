use crate::dict::kani_word::KaniWordDispatchEnum;

/// Sidecar enum for the heterogeneous `*split-map*` parts list. The
/// `def-simple-split` macro (`dict-split.lisp:13`) pushes three kinds
/// of element: a resolved word object, the bare keyword `:score`, or
/// the bare keyword `:pscore`.
#[derive(Debug, Clone)]
pub enum SplitPart {
    Word(KaniWordDispatchEnum),
    Score,
    PScore,
}
