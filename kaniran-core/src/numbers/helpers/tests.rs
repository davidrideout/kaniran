use super::*;

// --- _star_char_number_class_hash_star_ ---
/// Guards the explode-from-`CHAR_NUMBER_CLASS` build loop. 42 is
/// what the introspector captured from the live Lisp hashtable;
/// mismatches flag a regression in the loop, not a typo in the
/// source table.
#[test]
fn build_logic_produces_42_entries() {
    assert_eq!(char_number_class_hash().len(), 42);
}
