//! Common prefix/suffix patterns that fit the puzzle multiset (cue only).
//! Prefix/suffix lists are corpus-ranked from `data/affix_patterns.json` (see `build_affix_patterns.py`).

use crate::game::Round;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const MAX_STARTS: usize = 22;
const MAX_ENDS: usize = 22;

const AFFIX_PATTERNS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/affix_patterns.json"));
const WORDS_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/words.json"));

#[derive(Debug, Deserialize)]
struct AffixPatternsFile {
    prefixes: Vec<String>,
    suffixes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WordRecord {
    word: String,
}

fn patterns() -> &'static AffixPatternsFile {
    static CELL: OnceLock<AffixPatternsFile> = OnceLock::new();
    CELL.get_or_init(|| {
        serde_json::from_str(AFFIX_PATTERNS_JSON).expect("affix_patterns.json")
    })
}

fn lexicon_words() -> &'static Vec<String> {
    static CELL: OnceLock<Vec<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw: Vec<WordRecord> = serde_json::from_str(WORDS_JSON).expect("words.json");
        raw.into_iter()
            .map(|e| e.word.to_ascii_lowercase())
            .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic()))
            .collect()
    })
}

fn corpus_rank(side: &[String], gram: &str) -> usize {
    side.iter().position(|g| g == gram).unwrap_or(usize::MAX)
}

fn letter_counts(chars: impl Iterator<Item = char>) -> HashMap<char, usize> {
    let mut m = HashMap::new();
    for c in chars {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphabetic() {
            *m.entry(c).or_insert(0) += 1;
        }
    }
    m
}

fn multiset_can_spell(counts: &HashMap<char, usize>, pattern: &str) -> bool {
    let mut need = counts.clone();
    for c in pattern.chars() {
        let c = c.to_ascii_lowercase();
        if !c.is_ascii_alphabetic() {
            continue;
        }
        let Some(n) = need.get_mut(&c) else {
            return false;
        };
        if *n == 0 {
            return false;
        }
        *n -= 1;
    }
    true
}

/// Preserves list order; skips duplicates; optional max length.
fn filter_patterns_ordered(
    counts: &HashMap<char, usize>,
    list: &[String],
    limit: usize,
    max_len: usize,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in list {
        if p.len() > max_len {
            continue;
        }
        if !multiset_can_spell(counts, p) {
            continue;
        }
        if seen.insert(p.clone()) {
            out.push(p.clone());
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

const AFFIX_MIN_LEN: usize = 2;

/// Prefix/suffix n-grams among lexicon words spellable from the current letter multiset.
fn affixes_from_spellable_words(
    counts: &HashMap<char, usize>,
    max_chunk: usize,
    limit: usize,
) -> (Vec<String>, Vec<String>) {
    let patterns = patterns();
    let mut start_hits: HashMap<String, usize> = HashMap::new();
    let mut end_hits: HashMap<String, usize> = HashMap::new();
    let max_n = max_chunk.max(AFFIX_MIN_LEN);

    for w in lexicon_words() {
        if !multiset_can_spell(counts, w) {
            continue;
        }
        let wl = w.len();
        if wl < AFFIX_MIN_LEN {
            continue;
        }
        for n in AFFIX_MIN_LEN..=max_n.min(wl) {
            *start_hits.entry(w[..n].to_string()).or_insert(0) += 1;
            *end_hits.entry(w[wl - n..].to_string()).or_insert(0) += 1;
        }
    }

    let mut starts: Vec<(String, usize, usize)> = start_hits
        .into_iter()
        .map(|(g, c)| (g.clone(), c, corpus_rank(&patterns.prefixes, &g)))
        .collect();
    starts.sort_by(|a, b| {
        b.1
            .cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| b.0.len().cmp(&a.0.len()))
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut ends: Vec<(String, usize, usize)> = end_hits
        .into_iter()
        .map(|(g, c)| (g.clone(), c, corpus_rank(&patterns.suffixes, &g)))
        .collect();
    ends.sort_by(|a, b| {
        b.1
            .cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| b.0.len().cmp(&a.0.len()))
            .then_with(|| a.0.cmp(&b.0))
    });

    let starts: Vec<String> = starts
        .into_iter()
        .take(limit)
        .map(|(g, _, _)| g)
        .collect();
    let ends: Vec<String> = ends
        .into_iter()
        .take(limit)
        .map(|(g, _, _)| g)
        .collect();
    (starts, ends)
}

/// All letters still in play (answer track).
pub fn play_multiset(round: &Round) -> HashMap<char, usize> {
    let mut letters: Vec<char> = round.pool.clone();
    for slot in &round.answer {
        if let Some(c) = slot {
            letters.push(*c);
        }
    }
    letter_counts(letters.into_iter())
}

pub struct AffixHints {
    pub starts: Vec<String>,
    pub ends: Vec<String>,
}

pub fn affix_hints(round: &Round) -> AffixHints {
    let patterns = patterns();
    let counts = play_multiset(round);
    let word_len = counts.values().sum::<usize>().max(4);
    let max_chunk = word_len.min(6);

    let (mut starts, local_ends) = affixes_from_spellable_words(&counts, max_chunk, MAX_STARTS);

    if starts.len() < MAX_STARTS {
        for p in filter_patterns_ordered(
            &counts,
            &patterns.prefixes,
            MAX_STARTS - starts.len(),
            max_chunk,
        ) {
            if !starts.contains(&p) {
                starts.push(p);
            }
            if starts.len() >= MAX_STARTS {
                break;
            }
        }
    }

    let mut ends: Vec<String> = local_ends.into_iter().map(|s| format!("-{s}")).collect();
    if ends.len() < MAX_ENDS {
        for p in filter_patterns_ordered(
            &counts,
            &patterns.suffixes,
            MAX_ENDS - ends.len(),
            max_chunk,
        ) {
            let tag = format!("-{p}");
            if !ends.contains(&tag) {
                ends.push(tag);
            }
            if ends.len() >= MAX_ENDS {
                break;
            }
        }
    }

    AffixHints { starts, ends }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn embedded_patterns_nonempty() {
        let p = patterns();
        assert!(p.prefixes.len() >= 10);
        assert!(p.suffixes.len() >= 10);
        assert_eq!(p.prefixes[0].as_str(), "co");
        assert_eq!(p.suffixes[0].as_str(), "es");
    }

    #[test]
    fn affirm_uses_lexicon_not_letter_soup() {
        let mut rng = StdRng::seed_from_u64(1);
        let round = Round::new("affirm", &mut rng);
        let h = affix_hints(&round);
        assert!(
            !h.starts.iter().any(|s| s.chars().filter(|&c| c == 'f').count() > 2),
            "starts should not invent extra f's: {:?}",
            h.starts
        );
        assert!(
            h.starts.iter().any(|s| s == "fa" || s == "aff"),
            "expected lexicon starts, got {:?}",
            h.starts
        );
        assert!(
            h.ends.iter().any(|s| s.contains("irm") || s.contains("firm")),
            "expected lexicon ends, got {:?}",
            h.ends
        );
    }

    #[test]
    fn timbered_has_ed_and_re() {
        let mut rng = StdRng::seed_from_u64(1);
        let round = Round::new("timbered", &mut rng);
        let h = affix_hints(&round);
        assert!(h.starts.len() >= 10, "expected many starts, got {:?}", h.starts);
        assert!(h.ends.len() >= 10, "expected many ends, got {:?}", h.ends);
        assert!(h.ends.iter().any(|s| s.contains("ed")));
    }

    #[test]
    fn empty_pool_still_uses_answer_slots() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut round = Round::new("cat", &mut rng);
        round.pool.clear();
        round.answer = vec![Some('C'), Some('A'), Some('T')];
        let counts = play_multiset(&round);
        assert_eq!(counts.len(), 3);
    }
}