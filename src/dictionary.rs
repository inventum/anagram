//! Word list: embedded JSON, filter by length, random pick, clues.
//!
//! Public API: `Dictionary`, `WordEntry`, `random_word`, clue accessors.

use rand::seq::SliceRandom;
use rand::Rng;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClueKind {
    Definition { text: String },
    /// Highlights letters already in the right slot (see `GameState::highlight_correct_positions`).
    CorrectPositions,
    FirstLetter,
    VowelCount { text: String },
    Category { text: String },
}

pub type ClueEntry = ClueKind;

#[derive(Debug, Clone, Deserialize)]
pub struct WordEntry {
    pub word: String,
    #[serde(default)]
    pub definition: Option<String>,
    /// All WordNet noun-index glosses when available (see `clues_for`).
    #[serde(default)]
    pub noun_senses: Vec<String>,
    #[serde(default)]
    pub difficulty: Option<u8>,
    #[serde(default)]
    pub clues: Vec<ClueEntry>,
}

impl WordEntry {
    /// Playable words must have a real definition (not a generated fallback).
    pub fn has_definition(&self) -> bool {
        self.definition
            .as_ref()
            .is_some_and(|d| !d.is_empty())
    }
}

#[derive(Debug)]
pub struct Dictionary {
    entries: Vec<WordEntry>,
}

const WORDS_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/words.json"));

impl Dictionary {
    pub fn from_embedded() -> anyhow::Result<Self> {
        Self::from_json_str(WORDS_JSON)
    }

    fn from_json_str(json: &str) -> anyhow::Result<Self> {
        let raw: Vec<WordEntry> = serde_json::from_str(json)?;
        let mut by_word: std::collections::HashMap<String, WordEntry> =
            std::collections::HashMap::new();
        for e in raw {
            if !Self::is_valid_word(&e.word) {
                continue;
            }
            if !e.has_definition() {
                continue;
            }
            let key = e.word.to_ascii_lowercase();
            by_word
                .entry(key)
                .and_modify(|existing| {
                    if e.clues.len() > existing.clues.len() {
                        existing.clues.clone_from(&e.clues);
                    }
                    if e.definition.is_some() {
                        existing.definition.clone_from(&e.definition);
                    }
                    if e.noun_senses.len() > existing.noun_senses.len() {
                        existing.noun_senses.clone_from(&e.noun_senses);
                    }
                    if e.difficulty.is_some() {
                        existing.difficulty = e.difficulty;
                    }
                })
                .or_insert(e);
        }
        let mut entries: Vec<WordEntry> = by_word.into_values().collect();
        entries.sort_by(|a, b| a.word.cmp(&b.word));
        Ok(Self { entries })
    }

    fn is_valid_word(word: &str) -> bool {
        !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic())
    }

    pub fn words_with_length(&self, len: usize) -> Vec<&WordEntry> {
        self.entries
            .iter()
            .filter(|e| e.word.len() == len && Self::is_valid_word(&e.word))
            .collect()
    }

    pub fn random_word<'a>(
        &'a self,
        len: usize,
        rng: &mut impl Rng,
    ) -> Option<&'a WordEntry> {
        self.random_word_excluding(len, None, rng)
    }

    /// Pick a random word, avoiding `exclude` when other words exist at this length.
    pub fn random_word_excluding<'a>(
        &'a self,
        len: usize,
        exclude: Option<&str>,
        rng: &mut impl Rng,
    ) -> Option<&'a WordEntry> {
        let pool = self.words_with_length(len);
        let mut pick_pool: Vec<&WordEntry> = if let Some(ex) = exclude {
            pool.iter()
                .copied()
                .filter(|e| !e.word.eq_ignore_ascii_case(ex))
                .collect()
        } else {
            pool.clone()
        };
        if pick_pool.is_empty() {
            pick_pool = pool;
        }
        pick_pool.choose(rng).copied()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry_for_word(&self, word: &str) -> Option<&WordEntry> {
        let key = word.to_ascii_lowercase();
        self.entries
            .iter()
            .find(|e| e.word.eq_ignore_ascii_case(&key))
    }

    /// Known word with definition, or a minimal entry for custom puzzles.
    pub fn resolve_entry(&self, word: &str) -> WordEntry {
        if let Some(e) = self.entry_for_word(word) {
            return e.clone();
        }
        WordEntry {
            word: word.to_ascii_lowercase(),
            definition: Some(format!(
                "Custom puzzle — {}-letter word (not in bundled dictionary).",
                word.chars().count()
            )),
            noun_senses: vec![],
            difficulty: None,
            clues: vec![],
        }
    }

    /// Number of clues the player may request for this entry (Tab / [c]).
    pub fn clue_budget(entry: &WordEntry) -> usize {
        Self::clues_for(entry).len()
    }

    pub fn clue_budget_round(primary: &WordEntry, alternates: &[WordEntry]) -> usize {
        Self::clues_for_round(primary, alternates).len()
    }

    /// Clues for the round: definitions for the solution and same-multiset alternates, then position hints.
    pub fn clues_for_round(primary: &WordEntry, alternates: &[WordEntry]) -> Vec<ClueKind> {
        if primary.clues.len() >= MAX_STATIC_CLUES {
            return primary.clues.clone();
        }
        let mut clues = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<&WordEntry> = vec![primary];
        for alt in alternates {
            if !alt.word.eq_ignore_ascii_case(&primary.word) {
                entries.push(alt);
            }
        }
        for entry in entries {
            Self::push_definition_clues(entry, &mut clues, &mut seen);
        }
        if clues.is_empty() {
            let text = primary
                .definition
                .clone()
                .filter(|s| !s.is_empty())
                .expect("entries without definition are filtered at load");
            clues.push(ClueKind::Definition { text });
        }
        clues.push(ClueKind::CorrectPositions);
        clues.push(ClueKind::FirstLetter);
        clues
    }

    fn push_definition_clues(
        entry: &WordEntry,
        clues: &mut Vec<ClueKind>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if !entry.noun_senses.is_empty() {
            for text in entry.noun_senses.iter().take(MAX_NOUN_SENSE_CLUES) {
                if text.is_empty() {
                    continue;
                }
                if seen.insert(text.clone()) {
                    clues.push(ClueKind::Definition {
                        text: text.clone(),
                    });
                }
            }
        } else if let Some(def) = entry.definition.as_ref().filter(|s| !s.is_empty()) {
            if seen.insert(def.clone()) {
                clues.push(ClueKind::Definition {
                    text: def.clone(),
                });
            }
        }
    }

    /// Clues for this entry: noun senses (when present), then position / first-letter hints.
    pub fn clues_for(entry: &WordEntry) -> Vec<ClueKind> {
        if entry.clues.len() >= MAX_STATIC_CLUES {
            return entry.clues.clone();
        }
        let mut clues = Vec::new();
        if !entry.noun_senses.is_empty() {
            for text in entry.noun_senses.iter().take(MAX_NOUN_SENSE_CLUES) {
                if !text.is_empty() {
                    clues.push(ClueKind::Definition {
                        text: text.clone(),
                    });
                }
            }
        }
        if clues.is_empty() {
            let text = entry
                .definition
                .clone()
                .filter(|s| !s.is_empty())
                .expect("entries without definition are filtered at load");
            clues.push(ClueKind::Definition { text });
        }
        clues.push(ClueKind::CorrectPositions);
        clues.push(ClueKind::FirstLetter);
        clues
    }
}

const MAX_STATIC_CLUES: usize = 3;
const MAX_NOUN_SENSE_CLUES: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn from_embedded_loads_and_skips_invalid_entries() {
        let dict = Dictionary::from_embedded().expect("embedded words.json must parse");
        assert!(
            dict.entry_count() >= 15,
            "expected seeded list minus invalid rows"
        );
        for len in 4..=8 {
            assert!(
                dict.words_with_length(len).len() >= 3,
                "need at least 3 valid words of length {len}"
            );
        }
        assert!(dict.words_with_length(4).iter().all(|e| e.word.len() == 4));
    }

    #[test]
    fn from_json_skips_empty_and_non_alpha() {
        let json = r#"[
            {"word": "", "definition": "empty"},
            {"word": "tool", "definition": "implement"},
            {"word": "x1yz", "definition": "bad"},
            {"word": "alpha", "definition": "first letter"}
        ]"#;
        let dict = Dictionary::from_json_str(json).unwrap();
        assert_eq!(dict.entry_count(), 2);
        let words: Vec<_> = dict.entries.iter().map(|e| e.word.as_str()).collect();
        assert!(words.contains(&"tool"));
        assert!(words.contains(&"alpha"));
    }

    #[test]
    fn words_with_length_empty_when_no_matches() {
        let dict = Dictionary::from_embedded().unwrap();
        assert!(dict.words_with_length(99).is_empty());
    }

    #[test]
    fn random_word_none_when_empty_pool() {
        let dict = Dictionary::from_json_str(r#"[
            {"word": "only", "definition": "single"}
        ]"#)
        .unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        assert!(dict.random_word(5, &mut rng).is_none());
        assert!(dict.random_word(4, &mut rng).is_some());
    }

    #[test]
    fn random_word_excluding_prefers_different_word() {
        let json = r#"[
            {"word": "abed", "definition": "in bed"},
            {"word": "aced", "definition": "served an ace"},
            {"word": "aged", "definition": "grown old"}
        ]"#;
        let dict = Dictionary::from_json_str(json).unwrap();
        let mut rng = StdRng::seed_from_u64(99);
        let first = dict.random_word(4, &mut rng).unwrap();
        let second = dict
            .random_word_excluding(4, Some(&first.word), &mut rng)
            .unwrap();
        assert_ne!(
            first.word.to_ascii_lowercase(),
            second.word.to_ascii_lowercase()
        );
    }

    #[test]
    fn random_word_deterministic_with_seeded_rng() {
        let dict = Dictionary::from_embedded().unwrap();
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        let a = dict.random_word(5, &mut rng_a).map(|e| e.word.clone());
        let b = dict.random_word(5, &mut rng_b).map(|e| e.word.clone());
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn clue_kinds_deserialize() {
        let json = r#"{"word": "test", "clues": [
            {"kind": "definition", "text": "a trial"},
            {"kind": "first_letter"},
            {"kind": "vowel_count", "text": "1 vowel"},
            {"kind": "category", "text": "misc"}
        ]}"#;
        let entry: WordEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.clues.len(), 4);
        assert!(matches!(entry.clues[0], ClueKind::Definition { .. }));
        assert!(matches!(entry.clues[1], ClueKind::FirstLetter));
        assert!(matches!(entry.clues[2], ClueKind::VowelCount { .. }));
        assert!(matches!(entry.clues[3], ClueKind::Category { .. }));
        assert_eq!(Dictionary::clues_for(&entry).len(), 4);
    }

    #[test]
    fn noun_senses_become_definition_clues() {
        let json = r#"{"word":"pillar","definition":"sense0","noun_senses":["sense1","sense2"]}"#;
        let entry: WordEntry = serde_json::from_str(json).unwrap();
        let clues = Dictionary::clues_for(&entry);
        assert_eq!(clues.len(), 4); // 2 senses + positions + first letter
        assert!(matches!(
            &clues[0],
            ClueKind::Definition { text } if text == "sense1"
        ));
        assert!(matches!(
            &clues[1],
            ClueKind::Definition { text } if text == "sense2"
        ));
    }

    #[test]
    fn optional_difficulty_defaults() {
        let json = r#"{"word": "word", "clues": []}"#;
        let entry: WordEntry = serde_json::from_str(json).unwrap();
        assert!(entry.difficulty.is_none());
    }
}
