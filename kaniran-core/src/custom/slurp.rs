//! Port of `ichiran/custom:slurp` (gf — `dict-custom.lisp:7`).

use super::custom_source_class::CustomLoader;

pub fn slurp(loader: &mut CustomLoader) -> std::io::Result<usize> {
    match loader {
        // dict-custom.lisp:65 (defmethod slurp ((loader xml-loader)) ...)
        CustomLoader::Xml(x) => x.slurp()?,
        // dict-custom.lisp:88 (defmethod slurp ((loader csv-loader)) ...) + dict-custom.lisp:168 (defmethod slurp :after ((loader municipality-csv)) ...)
        CustomLoader::Municipality(m) => m.slurp()?,
        // dict-custom.lisp:271 (defmethod slurp ((loader ward-csv)) ...)
        CustomLoader::Ward(w) => w.slurp()?,
    }
    // dict-custom.lisp:10-12 (defmethod slurp :around (source) (call-next-method) (length (entries source)))
    Ok(loader.base().entries.len())
}
