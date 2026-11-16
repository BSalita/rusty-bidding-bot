//! The compiled-in pack's rules name only card fields the pack has. The
//! pack's `.test` files run in its own repository (`rbb bid test`).

use std::path::Path;

#[test]
fn card_references_are_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = rbb_engine::load_rules(&root.join("pack/pack.json")).unwrap();
    let errors = rbb_engine::check_card_refs(&rules.modules, rules.vocab.registry());
    assert!(errors.is_empty(), "\n{}", errors.join("\n"));
}

/// Every module's IR round-trips through JSON.
#[test]
fn modules_round_trip_through_json() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = rbb_engine::load_rules(&root.join("pack/pack.json")).unwrap();
    assert!(rules.modules.len() > 50);
    for module in &rules.modules {
        let json = bidspec::to_json(module);
        let back: bidspec::Module = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, module, "{}: JSON round trip", module.name);
    }
}
