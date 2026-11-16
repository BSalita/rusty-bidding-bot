//! The rules and stock cards, compiled into the binary.
//!
//! `PACK` is a convention pack's published snapshot (`pack/pack.json`, a copy
//! of bridge-common-conventions' `pack.json` without its test and notes
//! files): `rbb_engine::compile_snapshot(PACK.1, PACK.0)` gives the rule set
//! and the card vocabulary. `PACK_RULES_NUMBER` and `PACK_ID` say which rules
//! they are. `CARDS` holds the stock `.bbsa` cards (`cards/bbsa`). Nothing
//! here needs a filesystem at run time: this is what the release `rbb` and
//! the WASM build bid with when they are given no other rules.

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// The stock card called `name` (e.g. `21GF-DEFAULT`), as `.bbsa` text.
pub fn card(name: &str) -> Option<&'static str> {
    CARDS.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// The stock card called `name` as card JSON, where it has one
/// (`cards/json/`). Prefer it to [`card`]: it carries settings a `.bbsa`
/// cannot, such as Natural without Jacoby transfers.
pub fn card_json(name: &str) -> Option<&'static str> {
    CARD_JSON.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_cards_are_embedded() {
        assert_eq!(PACK.0, "pack/pack.json");
        assert!(PACK.1.contains("\"schema\": \"pack/v1\""));
        assert!(PACK_RULES_NUMBER > 0);
        assert!(PACK_ID.starts_with(&format!("{PACK_RULES_NUMBER}-")));
        assert!(card("21GF-DEFAULT").is_some());
        assert!(card_json("Natural").is_some());
        assert!(card_json("21GF-DEFAULT").is_none());
    }
}
