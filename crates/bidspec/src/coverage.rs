//! How much of a convention card our rules actually read.
//!
//! A card is a few hundred switches; the rules read a few dozen. The gap is
//! what the engine silently ignores when it bids that card, which is the
//! difference between "we disagree with BBA" and "we were never playing the
//! same system". Three buckets per setting that is switched on:
//!
//! - **read**: a module names it in a `card` line or a `param`, or a
//!   `[[derived]]` rule turns it into one that does;
//! - **ignored**: the field exists and nothing reads it;
//! - **unmapped**: the `.bbsa` key has no card field at all, so the setting
//!   never reaches our model (it is kept in `bba_passthrough` so an export
//!   round-trips). Only keys the card switches *on* are counted, to match
//!   the other two buckets.
//!
//! Carding, opening leads and free-text notes cannot change a call, so they
//! are counted apart from the conventions: the sections in `NOT_BIDDING`,
//! and any field marked `note = true` in `fields.toml` (a write-in line
//! inside a bidding section, such as `two_level.two_clubs.notes`).
//!
//! Used by `rbb card coverage` and by the WASM `coverage` call (the web
//! site's missing-convention flags).

use std::collections::BTreeSet;

use bridge_card::{Card, Value, Vocabulary};
use serde::Serialize;

use crate::Module;

/// Sections that describe the play or the card itself, not the bidding.
const NOT_BIDDING: [&str; 4] = ["carding", "leads", "notes", "metadata"];

#[derive(Debug, Clone, Serialize)]
pub struct Coverage {
    pub name: String,
    pub system: String,
    pub read: Vec<String>,
    pub ignored: Vec<String>,
    pub other: Vec<String>,
    pub unmapped: Vec<String>,
}

impl Coverage {
    /// Of the bidding settings this card switches on, the share our rules
    /// read.
    pub fn score(&self) -> f64 {
        let on = self.read.len() + self.ignored.len();
        if on == 0 {
            1.0
        } else {
            self.read.len() as f64 / on as f64
        }
    }
}

/// Every card path the rules name, directly or indirectly, by the name a
/// card lists it under. Indirectly:
///
/// - through a derivation in `vocab` (the rules' own vocabulary);
/// - with a convention pack, a convention whose rules read one of its
///   treatments or places (`<id>:over_1nt`) reads the convention's own
///   switch, and a rule that reads a function's part
///   (`defence_to_1nt.strong`) reads the setting that part replaced and
///   each convention filling that part that a module implements.
pub fn fields_read(modules: &[Module], vocab: &Vocabulary) -> BTreeSet<String> {
    // A rule, and the `.bbsa` map's derivations, may name a setting by any
    // of its spellings (a pack's address, or the card path it replaced); a
    // card lists it by the canonical one.
    let reg = vocab.registry();
    let canonical = |p: &str| reg.get(p).map_or_else(|| p.to_string(), |f| f.path.clone());
    let mut read: BTreeSet<String> = BTreeSet::new();
    for m in modules {
        read.extend(m.card.iter().map(|c| canonical(&c.path)));
        read.extend(m.params.iter().map(|p| canonical(&p.path)));
    }
    if let Some(pack) = vocab.pack() {
        let of_conventions: Vec<String> = read
            .iter()
            .filter_map(|p| p.split_once(':').map(|(id, _)| id.to_string()))
            .collect();
        read.extend(of_conventions);
        let implemented: BTreeSet<&str> = modules
            .iter()
            .flat_map(|m| m.skills.iter().map(|s| s.path.as_str()))
            .collect();
        for (address, setting) in pack.function_settings() {
            if !read.contains(&canonical(&address)) {
                continue;
            }
            read.insert(canonical(&setting));
            // The conventions that fill this part are chosen through it: one
            // a module implements is read when the part is.
            let Some((name, part)) = address.split_once('.') else {
                continue;
            };
            for (id, parts) in pack
                .function(name)
                .into_iter()
                .flat_map(|f| &f.implementers)
            {
                let fills = parts.is_empty() || parts.iter().any(|p| p == part);
                if fills && implemented.contains(id.as_str()) {
                    read.insert(id.clone());
                }
            }
        }
    }
    // A field feeding a derivation counts as read when the derived field is.
    for (inputs, outputs) in vocab.bbsa_derivations() {
        if outputs.iter().any(|o| read.contains(&canonical(o))) {
            read.extend(inputs.iter().map(|i| canonical(i)));
        }
    }
    read
}

/// Classify what `card` switches on against what `read` names.
pub fn of_card(
    name: &str,
    card: &Card,
    unmapped: Vec<String>,
    read: &BTreeSet<String>,
) -> Coverage {
    let reg = card.vocabulary().registry();
    let mut cov = Coverage {
        name: name.to_string(),
        system: match card.effective("general.system_category") {
            Some(Value::Text(s)) => s.clone(),
            _ => "two_over_one".into(),
        },
        read: Vec::new(),
        ignored: Vec::new(),
        other: Vec::new(),
        unmapped,
    };
    for (path, value) in card.values() {
        if !is_on(value) {
            continue;
        }
        // A field at its default says nothing about this card.
        if reg.get(path).and_then(|f| f.default.as_ref()) == Some(value) {
            continue;
        }
        let note = reg.get(path).is_some_and(|f| f.note);
        // A convention pack names a carding convention by its ID
        // (`defense/smith_echo`); the section is in its card schema 2 path,
        // which the pack keeps as a spelling.
        let spellings = reg.get(path).into_iter().flat_map(|f| f.aliases.iter());
        let not_bidding = std::iter::once(path)
            .chain(spellings.map(String::as_str))
            .any(|p| NOT_BIDDING.iter().any(|s| p.starts_with(s)));
        let bucket = if note || not_bidding {
            &mut cov.other
        } else if read.contains(path) {
            &mut cov.read
        } else {
            &mut cov.ignored
        };
        bucket.push(path.to_string());
    }
    cov
}

/// The settings of a card JSON that have no card field (`unknown`, from
/// its [`bridge_card::LoadReport`]) and are switched on: the JSON
/// counterpart of the `.bbsa` keys with no card field, for the
/// `unmapped` bucket.
pub fn unmapped_json(card: &Card, unknown: &[String]) -> Vec<String> {
    unknown
        .iter()
        .filter(|path| {
            card.unknown_value(path).is_some_and(|v| match v {
                serde_json::Value::Null => false,
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::String(s) => !s.trim().is_empty() && s != "none",
                serde_json::Value::Array(a) => !a.is_empty(),
                _ => true,
            })
        })
        .cloned()
        .collect()
}

/// Is this setting saying anything? `false`, an empty string and an unset
/// value are all "not switched on".
fn is_on(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Text(s) => !s.is_empty() && s != "none",
        Value::Int(_) => true,
        Value::Ints(v) => !v.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules(src: &str) -> Vec<Module> {
        vec![crate::parse(src, "test.bid").expect("parses")]
    }

    #[test]
    fn a_field_is_read_directly_or_through_a_derivation() {
        let read = fields_read(
            &modules(concat!(
                "module t \"t\"\n",
                "  card   notrump.stayman.play\n",
                "  param  minors = notrump.minor_transfers.style default relay\n",
                "\nafter 1N (P)\n  2C \"Stayman\" shows hcp>=8\n"
            )),
            crate::test_vocabulary(),
        );
        // By the name a card lists each under: the pack's address.
        assert!(read.contains("bidding_conventions/stayman"), "{read:?}");
        assert!(read.contains("bidding_conventions/minor_suit_transfers:style"));
        // A convention is read when one of its treatments is.
        assert!(read.contains("bidding_conventions/minor_suit_transfers"));
        // `minor_transfers` is derived from BBA's four switches, so those
        // count as read even though no rule names them.
        assert!(
            read.contains("bidding_conventions/minor_suit_transfers:two_s_clubs"),
            "derivation inputs count as read: {read:?}"
        );
        assert!(!read.contains("competitive_bidding/michaels"));
    }

    #[test]
    fn a_function_part_reads_its_setting_and_the_conventions_implemented() {
        let read = fields_read(
            &modules(concat!(
                "module t \"t\"\n",
                "  skill  competitive_bidding/cappelletti\n",
                "  param  def = defence_to_1nt.strong\n",
                "\nafter (1N) when def is competitive_bidding/cappelletti\n  2C \"x\"\n"
            )),
            crate::test_vocabulary(),
        );
        // What a card read from an older form lists the choice under.
        assert!(
            read.contains("competitive.vs_1nt_strong.system"),
            "{read:?}"
        );
        // The convention chosen through the part, which this module implements...
        assert!(read.contains("competitive_bidding/cappelletti"));
        // ...and not one that no module does.
        assert!(!read.contains("competitive_bidding/dont"));
    }

    #[test]
    fn settings_split_into_read_ignored_and_play_only() {
        let read = fields_read(
            &modules(concat!(
                "module t \"t\"\n",
                "  convention bidding_conventions/stayman\n",
                "\nafter 1N (P)\n  2C \"Stayman\" shows hcp>=8\n"
            )),
            crate::test_vocabulary(),
        );
        let mut card = Card::new(crate::test_vocabulary());
        card.set("notrump.stayman.play", Value::Bool(true)).unwrap();
        card.set("competitive.michaels.play", Value::Bool(true))
            .unwrap();
        card.set("carding.smith_echo", Value::Bool(true)).unwrap();
        // A write-in line (`note = true`) inside a bidding section.
        card.set("two_level.two_clubs.notes", Value::Text("Kokish".into()))
            .unwrap();
        card.set("competitive.ghestem.play", Value::Bool(false))
            .unwrap();
        let cov = of_card("t", &card, vec!["Some Key".into()], &read);
        assert_eq!(cov.read, ["bidding_conventions/stayman"]);
        assert_eq!(cov.ignored, ["competitive_bidding/michaels"]);
        assert_eq!(
            cov.other,
            ["defense/smith_echo", "two_level.two_clubs.notes"],
            "play and notes, not bidding"
        );
        assert_eq!(cov.unmapped.len(), 1);
        assert_eq!(cov.score(), 0.5);
    }
}
