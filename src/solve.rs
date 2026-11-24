//! CLI `solve` — stdout TSV, no TUI.

use crate::signature_index::{signature_from_str, LookupResult, SignatureIndex};
use std::io::{stdout, Write};

/// Exit code for `anagram solve` (SPEC §7.2).
pub const EXIT_OK: i32 = 0;
pub const EXIT_NO_MATCH: i32 = 1;
pub const EXIT_INVALID: i32 = 2;

/// Normalize scramble, lookup embedded index, print `word<TAB>definition` lines.
pub fn run_solve(scramble: &str) -> anyhow::Result<i32> {
    if signature_from_str(scramble).is_err() {
        return Ok(EXIT_INVALID);
    }

    let index = SignatureIndex::from_embedded()?;

    match index.lookup(scramble) {
        LookupResult::NoMatch => Ok(EXIT_NO_MATCH),
        LookupResult::Matches(mut entries) => {
            entries.sort_by(|a, b| a.word.cmp(&b.word));
            let mut out = stdout();
            for entry in entries {
                let def = entry.definition.as_deref().unwrap_or("");
                let _ = writeln!(out, "{}\t{}", entry.word, def);
            }
            let _ = out.flush();
            Ok(EXIT_OK)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_scramble_exits_2() {
        assert_eq!(run_solve("").unwrap(), EXIT_INVALID);
        assert_eq!(run_solve("ab1").unwrap(), EXIT_INVALID);
    }
}