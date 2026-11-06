#[test]
fn core_crate_is_linkable() {
    assert_eq!(acid_core::crate_name(), "acid-core");
}
