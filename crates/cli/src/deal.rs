//! `rbb deal`: dealer scripts (`.dlr`), read by dealer3's own parser.
//!
//! A convention's scenario is a dealer script: the hands that reach its
//! auctions. `rbb deal check` holds a script to the same standard as a rule
//! file, with the parser that will deal from it, so a pack's publish step
//! checks both with one tool.

use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

/// Every `.dlr` file under `path` (itself, when it is one).
pub fn collect(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
            collect(&entry?.path(), out)?;
        }
    } else if path.extension().is_some_and(|e| e == "dlr") {
        out.push(path.to_path_buf());
    }
    Ok(())
}

/// What is wrong with a script, if anything: the line (1-based) where the
/// parser places it, when it does, and the message.
pub fn problem(
    text: &str,
    params: &dealer_parser::ScriptParams,
) -> Option<(Option<usize>, String)> {
    let expanded = match dealer_parser::preprocess_all(text, params) {
        Ok(t) => t,
        Err(message) => return Some((None, message)),
    };
    match dealer_parser::parse_program(&expanded) {
        Ok(_) => None,
        Err(e) => Some((line_of(&e.message), first_line(&e.message))),
    }
}

/// The line in a parser message's ` --> 12:5` marker.
fn line_of(message: &str) -> Option<usize> {
    let at = message.find("--> ")? + 4;
    message[at..].split(':').next()?.trim().parse().ok()
}

/// What the parser expected, without the excerpt it draws under it.
fn first_line(message: &str) -> String {
    message
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('='))
        .map(|l| l.trim_start().trim_start_matches('=').trim().to_string())
        .unwrap_or_else(|| message.lines().next().unwrap_or("").trim().to_string())
}

/// `rbb deal check PATHS`: parse every script, report each problem as
/// `file:line: message`, and fail when there is one.
pub fn check(paths: &[PathBuf], params: &[String]) -> Result<()> {
    let mut given = dealer_parser::ScriptParams::default();
    for p in params {
        given.set(p)?;
    }
    let mut files = Vec::new();
    for p in paths {
        if !p.exists() {
            return Err(format!("{}: no such file or directory", p.display()).into());
        }
        collect(p, &mut files)?;
    }
    files.sort();
    let mut errors = 0;
    for file in &files {
        let text = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        if let Some((line, message)) = problem(&text, &given) {
            errors += 1;
            match line {
                Some(l) => eprintln!("{}:{l}: error: {message}", file.display()),
                None => eprintln!("{}: error: {message}", file.display()),
            }
        }
    }
    println!(
        "{} dealer script{}: {errors} error{}",
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        if errors == 1 { "" } else { "s" }
    );
    if errors > 0 {
        return Err(format!("{errors} dealer script(s) do not parse").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_script_has_no_problem() {
        let params = dealer_parser::ScriptParams::default();
        assert_eq!(
            problem("condition hcp(north) >= 15\nproduce 4\n", &params),
            None
        );
    }

    #[test]
    fn a_bad_script_is_placed_on_its_line() {
        let params = dealer_parser::ScriptParams::default();
        let (line, message) = problem("produce 4\ncondition hcp(north) >= >= 15\n", &params)
            .expect("the script does not parse");
        assert_eq!(line, Some(2));
        assert!(!message.is_empty());
    }
}
