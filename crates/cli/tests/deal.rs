//! `rbb deal`: a dealer script is checked, dealt from and bid by one tool.

use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbb-deal-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rbb(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rbb"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const SCRIPT: &str = "dealer north\nvulnerable none\n\
    condition hcp(north) >= 15 && hcp(north) <= 17 && shape(north, any 4333 + any 4432 + any 5332)\n\
    produce 5\n";

#[test]
fn check_passes_a_good_script_and_places_a_mistake_on_its_line() {
    let dir = scratch("check");
    std::fs::write(dir.join("good.dlr"), SCRIPT).unwrap();
    let (ok, out, _) = rbb(&["deal", "check", dir.to_str().unwrap()]);
    assert!(ok);
    assert!(out.contains("1 dealer script: 0 errors"), "{out}");

    std::fs::write(
        dir.join("bad.dlr"),
        "produce 4\ncondition hcp(north) >= >= 15\n",
    )
    .unwrap();
    let (ok, out, err) = rbb(&["deal", "check", dir.to_str().unwrap()]);
    assert!(!ok);
    assert!(err.contains("bad.dlr:2: error:"), "{err}");
    assert!(out.contains("2 dealer scripts: 1 error"), "{out}");
}

#[test]
fn run_deals_what_the_script_asks_for_the_same_for_a_seed_and_bid_pbn_bids_it() {
    let dir = scratch("run");
    let script = dir.join("one-nt.dlr");
    std::fs::write(&script, SCRIPT).unwrap();
    let deals = |seed: &str| {
        let (ok, out, err) = rbb(&["deal", "run", script.to_str().unwrap(), "--seed", seed]);
        assert!(ok, "{err}");
        out.lines()
            .filter(|l| l.starts_with("[Deal "))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let first = deals("42");
    assert_eq!(first.len(), 5);
    assert_eq!(first, deals("42"));
    assert_ne!(first, deals("43"));

    // The deals are boards the bidder reads: North, dealt 15 to 17 balanced,
    // opens 1NT on every one.
    let pbn = dir.join("deals.pbn");
    let bid = dir.join("bid.pbn");
    let (ok, _, err) = rbb(&[
        "deal",
        "run",
        script.to_str().unwrap(),
        "--seed",
        "42",
        "-o",
        pbn.to_str().unwrap(),
    ]);
    assert!(ok, "{err}");
    let (ok, _, err) = rbb(&[
        "bid-pbn",
        "-i",
        pbn.to_str().unwrap(),
        "-o",
        bid.to_str().unwrap(),
        "--ns-card",
        "21GF-DEFAULT",
    ]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&bid).unwrap();
    let openings: Vec<&str> = text
        .split("[Auction \"N\"]\n")
        .skip(1)
        .map(|rest| rest.split_whitespace().next().unwrap_or(""))
        .collect();
    assert_eq!(openings.len(), 5);
    assert!(
        openings.iter().all(|c| *c == "1NT" || *c == "1N"),
        "{openings:?}"
    );
}
