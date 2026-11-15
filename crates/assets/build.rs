//! Generates `$OUT_DIR/assets.rs`: the convention pack's published snapshot
//! (`pack/pack.json`, copied from bridge-common-conventions by
//! `scripts/sync_pack.py`), and every stock card `cards/bbsa/*.bbsa` (and
//! `cards/json/*.json` where a card has one), as `include_str!`s, so a
//! binary (the release `rbb`, the WASM build) carries rules to bid with and
//! needs no filesystem or network to load them. A host may still hand the
//! engine a newer snapshot.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

/// FNV-1a over the pack's text: identifies the compiled-in rules.
fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let cards_dir = root.join("cards/bbsa");
    let json_dir = root.join("cards/json");
    // A directory is scanned recursively for changes.
    println!("cargo:rerun-if-changed={}", cards_dir.display());
    println!("cargo:rerun-if-changed={}", json_dir.display());

    let mut cards = Vec::new();
    collect(&cards_dir, "bbsa", &mut cards);
    cards.sort();

    let mut out = String::new();
    // The convention pack's published snapshot (`pack/pack.json`, copied by
    // `scripts/sync_pack.py`): the rules rbb bids with when it is given no
    // others. Its id is the rules number and a hash of the text.
    let pack = root.join("pack/pack.json");
    println!("cargo:rerun-if-changed={}", pack.display());
    let pack_text = std::fs::read_to_string(&pack)
        .unwrap_or_else(|e| panic!("{}: {e} (python3 scripts/sync_pack.py)", pack.display()));
    let mut pack_hash: u64 = 0xcbf2_9ce4_8422_2325;
    fnv(&mut pack_hash, pack_text.as_bytes());
    // `"rules_number": 21,` near the top, written by the sync script.
    let number = pack_text
        .split("\"rules_number\":")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|n| n.trim().parse::<u64>().ok())
        .unwrap_or_else(|| panic!("{}: no rules_number", pack.display()));
    writeln!(
        out,
        "/// The convention pack's snapshot rbb bids with by default: (name, text).\npub static PACK: (&str, &str) = (\"pack/pack.json\", include_str!({:?}));\n\n/// The pack's rules number.\npub const PACK_RULES_NUMBER: u64 = {number};\n\n/// Identifies the compiled-in pack: its rules number and a hash of its text.\npub const PACK_ID: &str = \"{number}-{pack_hash:016x}\";\n",
        pack.display().to_string()
    )
    .unwrap();
    out.push_str("/// Stock cards: (name, .bbsa text).\n");
    out.push_str("pub static CARDS: &[(&str, &str)] = &[\n");
    for abs in &cards {
        let name = abs.file_stem().unwrap().to_string_lossy().into_owned();
        println!("cargo:rerun-if-changed={}", abs.display());
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            abs.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n\n");
    // Card JSON for the stock cards that have one: settings a .bbsa cannot
    // carry (Natural without Jacoby transfers, SAYC's 2S relay).
    let mut jsons = Vec::new();
    collect(&json_dir, "json", &mut jsons);
    jsons.sort();
    out.push_str("/// Stock cards that also come as card JSON: (name, JSON text).\n");
    out.push_str("pub static CARD_JSON: &[(&str, &str)] = &[\n");
    for abs in &jsons {
        let name = abs.file_stem().unwrap().to_string_lossy().into_owned();
        println!("cargo:rerun-if-changed={}", abs.display());
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            abs.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n\n");
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("assets.rs");
    std::fs::write(dest, out).unwrap();
}
