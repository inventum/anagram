# Letter-Signature Index — Product & Technical Specification

**Status:** Draft (not yet implemented)  
**Related:** [SPEC.md](./SPEC.md) (core TUI), `data/words.json`, `scripts/enrich_wordnet.py`  
**Version:** 1.2

---

## 1. Overview

**Purpose:** Precompute an **inverted index** from *letter multiset* (scrambled letters) → *all dictionary words* that anagram to that multiset, each with its stored meaning. Persist the index **once** at build time; at runtime, resolve a scramble in **O(1)** average time via hash map lookup.

**Problem solved today without this index:** The live game knows the **solution** when creating a round (`Round::new(word)`). If the product accepts **only a scramble** (or we want a dev/CLI “solve this anagram” tool), we must search the lexicon. Linear scan over all words is **O(N)** per query. The signature index makes lookup **O(1)** in N after a one-time **O(N · L)** build (L = word length, bounded by config max size).

**Non-goal:** This does not replace `data/words.json` as the source of truth for **random puzzle generation**, clues, and length-filtered picks. The index is a **derived artifact** built from the same (or a superset) lexical source.

---

## 2. Goals & Non-Goals

| Goals | Non-Goals |
|-------|-----------|
| Exact multiset match (A–Z, case-insensitive) | Fuzzy / typo-tolerant match |
| Zero or more words per scramble, each with definition when available | Picking a single “correct” word without user or ranking policy |
| One durable on-disk file, versioned with source data | Live WordNet / network lookup at query time |
| Deterministic rebuild from source JSON | Maintaining two independent word lists by hand |
| Unit-testable pure Rust API (`signature_key`, `lookup`) | Full TUI “scramble-only” mode in v1 of this spec (optional follow-on) |
| **Larger bundled lexicon** with definitions for solve + TUI clues (see §10) | Wiktionary-scale dumps in v1 of the lexicon expansion |

---

## 3. Concepts

### 3.1 Letter multiset

For a string `w` of ASCII letters, the **multiset** is the count of each letter `a`–`z`. Example: `INDIA` → `a=1, d=1, i=2, n=1`, all other counts 0.

**Invariants:**

- Only `a`–`z` after normalization (non-letters rejected at API boundary).
- Multiset **implies** length: `sum(counts) == len(w)`.

### 3.2 Signature key (canonical)

**v1 normative choice (resolved):** **`signature_string`** — all letters of the word in **lowercase sorted order**, e.g. `INDIA` → `"adiin"`, `LISTEN` / `SILENT` → `"eilnst"`.

| Property | Rule |
|----------|------|
| Derivation | `to_ascii_lowercase()`, keep `a`–`z` only, `sort_unstable()`, collect to `String` |
| JSON `buckets` keys | Exactly this string (see §5.2) |
| In-memory | `Signature` newtype wrapping `String` (or `Arc<str>`), `Hash + Eq` |

**Future (v2, optional):** `[u8; 26]` count vector or fixed-width count encoding for denser keys if bucket maps exceed ~50 MiB. v1 code MUST NOT mix two key encodings in one index file.

### 3.3 Bucket value

Each key maps to **zero or one bucket**; each bucket holds **one or more** lexical entries:

```json
{
  "word": "listen",
  "definition": "…",
  "difficulty": null
}
```

Order within bucket: **ascending `word`** (stable, deterministic). Optional future field: `frequency_rank` for disambiguation UI.

---

## 4. Build Pipeline

### 4.1 Source of truth

| Input | Role |
|-------|------|
| `data/words.json` | Primary input (same schema as today: `word`, `definition`, optional `clues`, `difficulty`) |
| Build pipeline output from §10 | **Required** for production lexicon: enlarged `words.json` before index generation |

**Rule:** The index is **always generated** from `words.json`; never hand-edited. `words.json` is **always generated** from approved sources in §10; never grown by hand.

**Lexicon size gate (required):** Release builds that ship `signature_index.json` MUST use a `words.json` that meets §10.3 minimum entry targets (or documented exception with written rationale in manifest).

### 4.2 Build steps

1. Load all `WordEntry` records from source JSON; filter invalid rows (same rules as `Dictionary::from_json_str`: alphabetic, non-empty, **must** `has_definition()` for the glossed production index). Spell-only tiers (§10.1 L6) use a **separate** source file and optional second index artifact — not mixed into the default glossed index.
2. For each entry, compute `signature_key(word)`.
3. Insert into map `key → Vec<WordEntry>` (append, dedupe by lowercase `word`).
4. Sort each bucket’s vector by `word`.
5. Write output artifact + manifest (§5).
6. Emit build stats: `entry_count`, `bucket_count`, `max_bucket_size`, `ambiguous_buckets` (size > 1).

### 4.3 Where build runs

| Option | Spec |
|--------|------|
| **A (v1 normative)** | `scripts/enrich_wordnet.py` (or `build_lexicon.py`) + **`scripts/build_signature_index.py`** — `make data` runs lexicon then index (see [RUNBOOK_SIGNATURE_INDEX.md](./RUNBOOK_SIGNATURE_INDEX.md)) |
| **B** | Rust binary `build_index` — optional later; must share golden tests with script output |
| **C** | `build.rs` — **not used** for WordNet refresh workflows |

### 4.4 Complexity

- Build: **O(N · L)** for N entries, L ≤ `max_size`.
- Storage size: **O(B + N)** where B = distinct signatures (B ≤ N).
- Query: **O(1)** map lookup + **O(k)** to return k matches in bucket (k typically small).

---

## 5. On-Disk Format

### 5.1 Files

| Path | Description |
|------|-------------|
| `data/signature_index.json` | Derived index; **includes** manifest fields at top level (§5.2) — no separate manifest file required for v1 |
| `data/words.manifest.json` | Lexicon build metadata (§10.1 L4): `source_name`, counts, `sha256` of `words.json` |
| `data/fixtures/words_mini.json` | Tiny lexicon for unit tests and parallel agent builds before M1 lands |

### 5.2 Index document (v1 schema)

```json
{
  "format_version": 1,
  "source": {
    "path": "data/words.json",
    "sha256": "<hex of words.json at build time>"
  },
  "locale": "en-US",
  "min_size": 4,
  "max_size": 10,
  "built_at": "2026-11-14T12:00:00Z",
  "entry_count": 5249,
  "bucket_count": 5100,
  "buckets": {
    "adiin": [
      { "word": "india", "definition": "…" }
    ],
    "eilnst": [
      { "word": "listen", "definition": "…" },
      { "word": "silent", "definition": "…" },
      { "word": "tinsel", "definition": "…" }
    ]
  }
}
```

**Notes:**

- `buckets` keys = **sorted signature string** (v1).
- Omit empty buckets (only non-empty keys).
- Definitions truncated to same max length as source pipeline (e.g. 280 chars).

### 5.3 Embedding

Mirror `words.json`:

```rust
const SIGNATURE_INDEX_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/signature_index.json"));
```

Optional future: load external path via CLI for huge lexicons.

### 5.4 Invalidation

At load time:

1. If embedded manifest `sha256` ≠ hash of bundled `words.json`, **warn** or **fail** (configurable; **fail in CI**, **warn in dev**).
2. `format_version` mismatch → hard error with rebuild instructions.

---

## 6. Runtime API (`signature_index` module)

### 6.1 Types

```rust
pub struct SignatureIndex {
    buckets: HashMap<Signature, Vec<WordEntry>>,
    // manifest fields for diagnostics
}

pub struct Signature; // private canonical key
```

### 6.2 Functions

| Function | Behavior |
|----------|----------|
| `signature_from_str(s: &str) -> Result<Signature, SignatureError>` | Normalize to lowercase a–z; reject empty / invalid chars |
| `SignatureIndex::from_embedded() -> Result<Self>` | Parse `data/signature_index.json` |
| `lookup(&self, scramble: &str) -> LookupResult` | Same multiset as scramble → clone bucket entries |
| `lookup_signature(&self, sig: &Signature) -> &[WordEntry]` | Internal/testing |

**`LookupResult`:**

| Variant | Meaning |
|---------|---------|
| `Matches(Vec<WordEntry>)` | k ≥ 1 |
| `NoMatch` | k = 0 |

**Entries without definition:** If source allows spelling-only rows in a future lexicon, include them in the bucket with `definition: null`; TUI shows “no gloss” for that row. Current WordNet pipeline always has definitions for indexed play words.

### 6.3 Relationship to `Dictionary`

| Concern | `Dictionary` (today) | `SignatureIndex` |
|---------|----------------------|------------------|
| Random word by length | `random_word_excluding` | Not used |
| Clues for known solution | `clues_for`, `resolve_entry` | Not used |
| Scramble → candidates | N/A | `lookup` |
| Custom user word | `resolve_entry` | Optional: `lookup` if user entered scramble instead of solution (future UX) |

Both share `WordEntry` from `dictionary`. Implement load/lookup in **`src/signature_index.rs`** (re-export from `lib.rs`); keep `Dictionary` focused on random-round selection.

---

## 7. Product Surfaces (phased)

### 7.1 v1 — Library + tests only

- Build script emits `signature_index.json`.
- Rust load + lookup API.
- No TUI change required for shipping v1 of this spec.

### 7.2 v1.1 — CLI solve (recommended first UX)

```bash
anagram solve SILENT   # or: anagram solve --scramble EILNTS
# stdout: one line per match: word + definition (tab-separated or JSON lines)
```

Exit codes: `0` if ≥1 match, `1` if no match, `2` if invalid input.

### 7.3 v2 — TUI / custom puzzle

- **Scramble input mode:** User types pool letters only; app lists matches or auto-starts if unique match (policy flag).
- **Disambiguation:** If `|bucket| > 1`, show numbered list; user picks intended solution → then normal `Round` (solution known, reshuffle pool).

### 7.4 Ranking (optional)

When k > 1, sort bucket by:

1. Word in bundled allowlist / frequency file
2. Alphabetical tie-break

Document chosen policy in config: `disambiguation = "alphabetic" | "frequency"`.

---

## 8. Correctness & Edge Cases

| Case | Expected behavior |
|------|-------------------|
| Scramble length mismatch | Still valid if multiset matches; length = sum of counts |
| Case | `Listen` and `LISTEN` same key |
| Duplicate source words | Dedupe by lowercase `word` in bucket |
| Same word twice in source JSON | Keep one (longer definition wins — same merge as `Dictionary::from_json_str`) |
| Scramble not in lexicon | `NoMatch` |
| Multiset with multiple answers | All listed; meanings may differ |
| Non-ASCII | Reject at `signature_from_str` (v1 English a–z only) |

---

## 9. Unit Tests (required)

| Test | Detail |
|------|--------|
| `signature_key_known_vectors` | e.g. `INDIA` → key equals `adiin`; `LISTEN` / `SILENT` share key |
| `build_deterministic` | Two builds from same `words.json` → identical `sha256` of output |
| `lookup_known_anagram` | Scramble `eilnst` (any order, e.g. `SILENT`) → contains `listen` and `silent` if both in source |
| `lookup_no_match` | `zzzz` → empty |
| `load_embedded` | `from_embedded()` parses shipped file |
| `manifest_mismatch` | Optional test with tampered hash |
| `max_bucket_size` | Sanity bound (e.g. < 100) on bundled index to catch build bugs |

Use small fixture JSON in `#[cfg(test)]`, not full 5k file.

---

## 10. Expanded Lexicon (Required)

The signature index is only as useful as `words.json`. Shipping the index feature **requires** a deliberate **lexicon expansion** so scramble lookup and gameplay cover far more English than the current ~5k **common ∩ WordNet** slice.

The index size scales with **distinct signatures** (≤ number of words), not marketing name alone.

### 10.1 Requirements

| ID | Requirement |
|----|-------------|
| **L1** | Single automated pipeline produces `data/words.json` (documented in `scripts/` + [RUNBOOK_SIGNATURE_INDEX.md](./RUNBOOK_SIGNATURE_INDEX.md)). No manual JSON edits for routine releases. |
| **L2** | Every indexed **play/solve** entry MUST be ASCII `a`–`z` only and respect configurable **min/max length** (default **4–10**, aligned with `AppConfig`). |
| **L3** | Every entry in the **glossed tier** (default for bundled game + solve CLI) MUST have a non-empty `definition` string (same rule as `WordEntry::has_definition()`). |
| **L4** | Lexicon build writes **`data/words.manifest.json`**; index build copies `source.sha256` of `words.json` into **`signature_index.json`** (§5.2). Both record `built_at`, `entry_count`. |
| **L5** | **Attribution** for lexical data MUST ship in repo (`data/ATTRIBUTION.md` or `docs/ATTRIBUTION.md`) and be referenced from README. |
| **L6** | Optional **spell-only tier** (separate file or flag) MAY add anagram targets without glosses; MUST NOT be mixed into glossed tier without `definition: null` and documented behavior (§6.2). |

### 10.2 Recommended primary source (MUST use for v1 expansion)

**Princeton WordNet 3.0+** — official download: [https://wordnet.princeton.edu/download](https://wordnet.princeton.edu/download)

**Why (recommended default):**

- Already integrated in `scripts/enrich_wordnet.py` (lemma index + gloss parsing).
- Definitions match TUI “definition clue” needs.
- Permissive **WordNet license**; suitable for bundling with attribution.
- Expanding **lemma coverage** in length range is the largest gain without changing file schema.

**Required pipeline change (v1 expansion):**

1. Download / unpack WordNet `dict/` (same layout as today’s `WN_DIR`).
2. Build candidate set from **WordNet lemmas** (all POS) in `min_size..=max_size`, not from `iter_system_words() ∩ allowlist` only.
3. Apply **quality filters** (below), not the current hard **Google 10k allowlist intersection**.
4. Write `data/words.json`; then build `data/signature_index.json` (§4).

**Quality filters (retain after moving off allowlist):**

| Filter | Purpose |
|--------|---------|
| Lemma is single-token, alphabetic | Exclude phrases with spaces/underscores unless explicitly normalized |
| Has WordNet gloss for lemma (incl. exception table) | Satisfies L3 |
| Optional: drop lemmas with `_` multiword forms in v1 | Reduce noise |
| Optional **frequency floor** | See §10.4 |

**Do not use as primary glossed source for v1:** raw `/usr/share/dict/words` alone — no definitions; spell-only tier only.

### 10.3 Size targets

Targets are for **glossed** `words.json` (length 4–10). Adjust manifest if config bounds change.

| Phase | `entry_count` (approx.) | Notes |
|-------|-------------------------|-------|
| **Current baseline** | ~5k | `allowlist ∩ system ∩ WordNet` |
| **M1 (required for index v1 ship)** | **≥ 35k** (or documented floor from first `wordnet-full` dry run if corpus is lower) | Full WordNet lemmas in range with gloss, no 10k allowlist; set `ANAGRAM_LEXICON_MIN_ENTRIES` from measured count before release |
| **M2 (recommended)** | **50k–80k** | M1 + optional lemma forms via exception table; dedupe aggressively |
| **M3 (future)** | 100k+ | Secondary merge or spell-only tier |

CI or release script MUST **fail** if glossed `entry_count` < **M1** when building production data artifacts (override via env `ANAGRAM_LEXICON_MIN_ENTRIES` for experiments only).

### 10.4 Optional supplements (SHOULD not replace WordNet for glosses)

Use only to **extend solve coverage** or **disambiguation**, merged in build with explicit provenance.

| Source | Use | License / notes |
|--------|-----|-----------------|
| **`data/common_allowlist.txt`** (Google 10k) | **Frequency ranking** when a signature bucket has multiple words; optional filter “prefer common for random puzzle” | Check upstream terms; list is not a dictionary |
| **[SCOWL](http://wordlist.aspell.net/)** | Spell-only tier or “is this English-ish?” filter | Size levels documented on site |
| **`/usr/share/dict/web2`** (system) | Spell-only tier for macOS/Linux dev builds | No gloss |
| **[dwyl/english-words](https://github.com/dwyl/english-words)** | Large spelling list for solve-only index file | MIT; verify before redistributing merged artifact |

**Not recommended for bundled redistribution without legal review:** ENABLE/YAWL/Scrabble tournament lists.

**Not recommended for v1 glossed bulk:** full **Wiktionary** / Kaikki JSON — valuable at M3 but heavy ETL and **CC BY-SA** share-alike obligations on derived `words.json`.

### 10.5 `enrich_wordnet.py` contract (normative)

The enrichment script (or successor `build_lexicon.py`) MUST support:

| Flag / config | Behavior |
|---------------|----------|
| `--mode common` | Legacy: allowlist ∩ system (regression / small fixtures) |
| `--mode wordnet-full` | **Production:** all WordNet lemmas in length range with gloss (M1) |
| `--min-len` / `--max-len` | Default 4 / 10 |
| `--allowlist PATH` | Optional **ranking or soft filter**, not required intersection in `wordnet-full` |
| `--emit-manifest` | Write sidecar with counts and hashes |

### 10.6 Workflow

```text
WordNet dict/  →  enrich (wordnet-full)  →  data/words.json
data/words.json  →  build_signature_index  →  data/signature_index.json
cargo test  →  commit data + attribution  →  release
```

### 10.7 Memory implications (informative)

M1 (~35k–80k) glossed entries typically yield a **single-digit to low tens of MiB** index on disk and in RAM when fully loaded (§Overview, implementation notes). If M3 exceeds ~50 MiB on disk, implementation SHOULD switch to SQLite/mmap per runtime discussion — record actual `index_bytes` in manifest after first M1 build.

---

## 11. Security & Licensing

- Index is derived from `words.json`; same attribution obligations as WordNet / source data.
- No user-provided strings in the build map without validation (build is offline).
- CLI `solve` must not echo unscrambled lists to logs in sensitive environments (N/A for local game).

---

## 12. Acceptance Criteria

1. Lexicon pipeline implements **`wordnet-full` mode** and produces `words.json` with **`entry_count` ≥ M1** (§10.3).
2. `data/ATTRIBUTION.md` (or equivalent) cites **WordNet** and build tooling.
3. `scripts/` (or documented `make` target) builds `data/signature_index.json` from current `data/words.json`.
4. `SignatureIndex::from_embedded()` loads in < 500 ms on typical hardware for M1-sized artifacts (measure and tighten after first build).
5. `lookup(scramble)` returns all and only words whose multisets equal `scramble`.
6. `cargo test` includes §9 tests plus at least one fixture built from **>1 word sharing a signature** (ambiguous bucket).
7. [RUNBOOK_SIGNATURE_INDEX.md](./RUNBOOK_SIGNATURE_INDEX.md) documents parallel implementation, WordNet path, `wordnet-full`, and index regeneration.

---

## 13. Open Questions

1. ~~**Key encoding**~~ — **Resolved:** `signature_string` (§3.2).
2. **Commit index:** Recommend **commit** `data/signature_index.json` + `data/words.json` for reproducible builds; CI may rebuild to verify hashes match manifest.
3. **Single match auto-start:** Default on or off in TUI v2.
4. **Spell-only companion file:** `signature_index_spell.json` at M2 vs single merged bucket with null definitions.
5. **Frequency file:** Ship static ranks from Google 1-grams vs 10k allowlist order only.
6. **M1 floor:** If first `wordnet-full` count is below 35k, update `ANAGRAM_LEXICON_MIN_ENTRIES` and this spec’s M1 row to the measured value before release.

---

## Appendix A — Example lookup flow

```
Scramble input: "EILNTS"   (letters only; order arbitrary)
    → normalize letters: "eilnts"
    → signature_key (sorted): "eilnst"
    → buckets["eilnst"] → [ listen, silent, tinsel, ... ]  (if present in words.json)
    → render each (word, definition)
```

---

## Appendix B — Normative `signature_key` (v1)

```
fn signature_key(word: &str) -> String {
    let mut chars: Vec<char> = word
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    chars.sort_unstable();
    chars.into_iter().collect()
}
```

Equality of multisets ⟺ equal sorted signature for a–z inputs.