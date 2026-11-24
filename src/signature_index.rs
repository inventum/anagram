//! Letter-signature index: scramble → candidate words.
//!
//! Wave 0: public types and stub functions (Track SK / RI implement behavior).

use crate::dictionary::WordEntry;
use serde::Deserialize;
use std::collections::HashMap;

const SIGNATURE_INDEX_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/signature_index.json"));

/// Canonical bucket key (sorted lowercase letters); normative derivation in Track SK.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature(String);

/// Why a scramble or word could not be normalized to a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    Empty,
    InvalidCharacter { ch: char },
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureError::Empty => write!(f, "empty input"),
            SignatureError::InvalidCharacter { ch } => {
                write!(f, "invalid character {:?}", ch)
            }
        }
    }
}

impl std::error::Error for SignatureError {}

/// Outcome of resolving a scramble against the index.
#[derive(Debug, Clone)]
pub enum LookupResult {
    Matches(Vec<WordEntry>),
    NoMatch,
}

#[derive(Debug, Clone, Deserialize)]
struct IndexSource {
    #[allow(dead_code)]
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct IndexDocument {
    format_version: u32,
    source: IndexSource,
    #[allow(dead_code)]
    locale: String,
    #[allow(dead_code)]
    min_size: u32,
    #[allow(dead_code)]
    max_size: u32,
    #[allow(dead_code)]
    built_at: String,
    #[allow(dead_code)]
    entry_count: u64,
    #[allow(dead_code)]
    bucket_count: u64,
    buckets: HashMap<String, Vec<WordEntry>>,
}

/// Precomputed multiset → words map (loaded from `data/signature_index.json`).
#[derive(Debug)]
pub struct SignatureIndex {
    buckets: HashMap<String, Vec<WordEntry>>,
}

/// v1 normative key: sorted lowercase `a`–`z` letters (SPEC §3.2, Appendix B).
pub fn signature_key(word: &str) -> String {
    let mut chars: Vec<char> = word
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    chars.sort_unstable();
    chars.into_iter().collect()
}

/// Normalize user input to a `Signature` (SPEC §6.2).
pub fn signature_from_str(s: &str) -> Result<Signature, SignatureError> {
    if s.is_empty() {
        return Err(SignatureError::Empty);
    }
    let mut chars = Vec::new();
    for c in s.chars() {
        if !c.is_ascii_alphabetic() {
            return Err(SignatureError::InvalidCharacter { ch: c });
        }
        chars.push(c.to_ascii_lowercase());
    }
    chars.sort_unstable();
    Ok(Signature(chars.into_iter().collect()))
}

impl Signature {
    /// Canonical sorted key string (bucket map key).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SignatureIndex {
    /// Parse embedded `data/signature_index.json` (SPEC §6.2).
    pub fn from_embedded() -> anyhow::Result<Self> {
        Self::from_str(SIGNATURE_INDEX_JSON)
    }

    /// Parse a §5.2 index document (tests and tooling).
    pub fn from_str(json: &str) -> anyhow::Result<Self> {
        let doc: IndexDocument = serde_json::from_str(json)?;
        Self::from_document(doc)
    }

    fn from_document(doc: IndexDocument) -> anyhow::Result<Self> {
        if doc.format_version != 1 {
            anyhow::bail!(
                "unsupported signature index format_version {} (expected 1); rebuild with scripts/build_signature_index.py",
                doc.format_version
            );
        }
        let _ = &doc.source.sha256;
        Ok(Self {
            buckets: doc.buckets,
        })
    }

    /// All lexicon entries sharing the scramble's letter multiset.
    pub fn lookup(&self, scramble: &str) -> LookupResult {
        let Ok(sig) = signature_from_str(scramble) else {
            return LookupResult::NoMatch;
        };
        match self.buckets.get(sig.as_str()) {
            Some(entries) if !entries.is_empty() => LookupResult::Matches(entries.clone()),
            _ => LookupResult::NoMatch,
        }
    }

    /// Direct bucket access for tests and diagnostics.
    pub fn lookup_signature(&self, sig: &Signature) -> &[WordEntry] {
        self.buckets
            .get(sig.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_MINI: &str =
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/fixtures/signature_index_mini.json"
        ));

    #[test]
    fn signature_key_known_vectors() {
        assert_eq!(signature_key("INDIA"), "adiin");
        assert_eq!(signature_key("listen"), signature_key("SILENT"));
        assert_eq!(signature_key("listen"), "eilnst");
    }

    #[test]
    fn signature_from_str_matches_signature_key_for_letters_only() {
        let sig = signature_from_str("Listen").expect("valid letters");
        assert_eq!(sig.as_str(), signature_key("Listen"));
        assert_eq!(sig.as_str(), "eilnst");
    }

    #[test]
    fn signature_from_str_rejects_empty() {
        assert_eq!(signature_from_str(""), Err(SignatureError::Empty));
    }

    #[test]
    fn signature_from_str_rejects_invalid_character() {
        let err = signature_from_str("ab1").unwrap_err();
        assert_eq!(err, SignatureError::InvalidCharacter { ch: '1' });
        let err = signature_from_str("café").unwrap_err();
        assert_eq!(err, SignatureError::InvalidCharacter { ch: 'é' });
    }

    #[test]
    fn from_str_loads_fixture() {
        let index = SignatureIndex::from_str(FIXTURE_MINI).expect("fixture parse");
        assert_eq!(index.buckets.len(), 2);
        let sig = signature_from_str("india").expect("valid");
        assert_eq!(index.lookup_signature(&sig).len(), 1);
        assert_eq!(index.lookup_signature(&sig)[0].word, "india");
    }

    #[test]
    fn lookup_ambiguous_eilnst_bucket() {
        let index = SignatureIndex::from_str(FIXTURE_MINI).expect("fixture parse");
        let result = index.lookup("SILENT");
        let LookupResult::Matches(entries) = result else {
            panic!("expected matches for SILENT");
        };
        let words: Vec<&str> = entries.iter().map(|e| e.word.as_str()).collect();
        assert_eq!(words.len(), 2);
        assert!(words.contains(&"listen"));
        assert!(words.contains(&"silent"));

        let sig = signature_from_str("listen").expect("valid");
        let bucket = index.lookup_signature(&sig);
        assert_eq!(bucket.len(), 2);
    }

    #[test]
    fn lookup_no_match() {
        let index = SignatureIndex::from_str(FIXTURE_MINI).expect("fixture parse");
        assert!(matches!(index.lookup("zzzz"), LookupResult::NoMatch));
    }

    #[test]
    fn from_embedded_loads_production_index() {
        let index = SignatureIndex::from_embedded().expect("embedded index must parse");
        assert!(index.buckets.len() > 1000);
    }
}