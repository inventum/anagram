#!/usr/bin/env python3
"""Build glossed word list from Princeton WordNet.

WordNet dict layout: unpack the official download so ``dict/`` contains
``data.noun``, ``index.noun``, ``*.exc``, etc. Default path: ``/tmp/wn/dict``.
Override with env ``WN_DIR`` or point at another ``dict/`` tree.

  curl -LO https://wordnetcode.princeton.edu/wn3.1.dict.tar.gz
  mkdir -p /tmp/wn && tar -xzf wn3.1.dict.tar.gz -C /tmp/wn
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_WN_DIR = Path("/tmp/wn/dict")
SYSTEM_DICT = Path("/usr/share/dict/words")
DATA_FILES = ("data.noun", "data.verb", "data.adj", "data.adv")
INDEX_FILES = ("index.noun", "index.verb", "index.adj", "index.adv")
EXC_FILES = ("noun.exc", "verb.exc", "adj.exc", "adv.exc")
REQUIRED_WN_FILES = (
    "data.noun",
    "index.noun",
    "index.verb",
    "index.adj",
    "index.adv",
)


def resolve_wn_dir() -> Path:
    raw = os.environ.get("WN_DIR", "").strip()
    return Path(raw) if raw else DEFAULT_WN_DIR


def wordnet_dict_ready(wn_dir: Path) -> bool:
    """Princeton layout: ``data.*`` and ``index.*`` flat under ``dict/``."""
    if not wn_dir.is_dir():
        return False
    return all((wn_dir / name).is_file() for name in REQUIRED_WN_FILES)


def missing_wordnet_files(wn_dir: Path) -> list[str]:
    return [name for name in REQUIRED_WN_FILES if not (wn_dir / name).is_file()]


def parse_glosses(wn_dir: Path) -> dict[int, str]:
    offset_gloss: dict[int, str] = {}
    for name in DATA_FILES:
        path = wn_dir / name
        if not path.exists():
            continue
        with path.open(encoding="ascii", errors="ignore") as f:
            for line in f:
                if not line.strip() or line.startswith(" "):
                    continue
                if "|" not in line:
                    continue
                head, gloss_part = line.split("|", 1)
                head_fields = head.split()
                try:
                    off = int(head_fields[0])
                except ValueError:
                    continue
                gloss = gloss_part.strip().split(";")[0].strip()
                if gloss:
                    offset_gloss.setdefault(off, gloss)
    return offset_gloss


def parse_noun_synset_meta(
    wn_dir: Path,
) -> tuple[dict[int, int], dict[int, list[str]]]:
    """Synset offset → lexicographer file id and member word forms (from ``data.noun``)."""
    path = wn_dir / "data.noun"
    lex: dict[int, int] = {}
    members: dict[int, list[str]] = {}
    if not path.exists():
        return lex, members
    with path.open(encoding="ascii", errors="ignore") as f:
        for line in f:
            if not line.strip() or line[0] in " \t":
                continue
            head = line.split("|", 1)[0]
            fields = head.split()
            if len(fields) < 5:
                continue
            try:
                off = int(fields[0])
                lex_id = int(fields[1])
                wcnt = int(fields[3])
            except ValueError:
                continue
            lex[off] = lex_id
            words: list[str] = []
            idx = 4
            for _ in range(wcnt):
                if idx >= len(fields):
                    break
                words.append(fields[idx])
                idx += 2
            members[off] = words
    return lex, members


def parse_noun_lemma_senses(wn_dir: Path, offset_gloss: dict[int, str]) -> dict[str, list[str]]:
    """All noun-index glosses per lemma (one entry per synset offset)."""
    senses, _ = parse_noun_lemma_index(wn_dir, offset_gloss)
    return senses


def parse_noun_lemma_index(
    wn_dir: Path, offset_gloss: dict[int, str]
) -> tuple[dict[str, list[str]], dict[str, list[int]]]:
    """Noun lemmas → gloss lists and WordNet synset offsets."""
    path = wn_dir / "index.noun"
    senses: dict[str, list[str]] = {}
    offsets: dict[str, list[int]] = {}
    if not path.exists():
        return senses, offsets
    with path.open(encoding="ascii", errors="ignore") as f:
        for line in f:
            if not line.strip() or line.startswith(" "):
                continue
            fields = line.split()
            if len(fields) < 4:
                continue
            lemma = fields[0].replace("_", " ").lower()
            try:
                synset_cnt = int(fields[2])
                off_list = [int(x) for x in fields[-synset_cnt:]]
            except ValueError:
                continue
            glosses: list[str] = []
            for off in off_list:
                gloss = offset_gloss.get(off)
                if gloss:
                    glosses.append(gloss[:280])
            if glosses:
                senses[lemma] = glosses
                offsets[lemma] = off_list
    return senses, offsets


def _lemma_synset_surface_title_case(lemma: str, forms: list[str]) -> bool:
    for raw in forms:
        surface = raw.replace("_", " ")
        if surface.lower() != lemma:
            continue
        if surface and surface[0].isupper():
            return True
    return False


def is_proper_name_noun_synset(
    lemma: str,
    offset: int,
    synset_lex: dict[int, int],
    synset_words: dict[int, list[str]],
) -> bool:
    """Geo / named entities vs common nouns (``balker`` stays inflectable)."""
    lex_id = synset_lex.get(offset)
    if lex_id is None:
        return False
    if lex_id == 15:  # noun.location
        return True
    if lex_id in (18, 19):  # person, group — title-case member only
        return _lemma_synset_surface_title_case(lemma, synset_words.get(offset, []))
    return False


def skip_noun_plural(
    lemma: str,
    offsets: list[int],
    synset_lex: dict[int, int],
    synset_words: dict[int, list[str]],
) -> bool:
    """No ``baghdad`` → ``baghdads``; still allow ``balker`` → ``balkers``."""
    if not offsets:
        return True
    return all(
        is_proper_name_noun_synset(lemma, off, synset_lex, synset_words) for off in offsets
    )


def inherit_noun_senses(
    form: str, noun_by_lemma: dict[str, list[str]], exc: dict[str, str]
) -> list[str]:
    w = form.lower()
    if w in noun_by_lemma:
        return noun_by_lemma[w]
    base = exc.get(w)
    if base and base in noun_by_lemma:
        return noun_by_lemma[base]
    if w.endswith("es") and w[:-2] in noun_by_lemma:
        return noun_by_lemma[w[:-2]]
    if w.endswith("s") and w[:-1] in noun_by_lemma:
        return noun_by_lemma[w[:-1]]
    return []


def expand_cross_reference_gloss(
    word: str, gloss: str, lemma_def: dict[str, str]
) -> str:
    """WordNet sometimes glosses a lemma as only another word's name (e.g. smyrnium → "Alexanders")."""
    g = gloss.strip()
    if not g or " " in g:
        return gloss
    ref = g.lower().replace("_", " ")
    if ref == word:
        return gloss
    full = lemma_def.get(ref)
    if full and len(full) > len(g) + 8:
        return full
    return gloss


def polish_entry_glosses(entry: dict[str, object], lemma_def: dict[str, str]) -> None:
    word = str(entry["word"])
    if defn := entry.get("definition"):
        entry["definition"] = expand_cross_reference_gloss(word, str(defn), lemma_def)[:280]
    senses = entry.get("noun_senses")
    if isinstance(senses, list) and senses:
        seen: set[str] = set()
        polished: list[str] = []
        for s in senses:
            text = expand_cross_reference_gloss(word, str(s), lemma_def)[:280]
            if text not in seen:
                seen.add(text)
                polished.append(text)
        entry["noun_senses"] = polished
        entry["definition"] = polished[0]


def attach_noun_senses(
    entry: dict[str, object], noun_by_lemma: dict[str, list[str]], exc: dict[str, str]
) -> None:
    word = str(entry["word"])
    senses = noun_by_lemma.get(word) or inherit_noun_senses(word, noun_by_lemma, exc)
    if senses:
        entry["noun_senses"] = senses
        entry["definition"] = senses[0]


def parse_lemma_index(wn_dir: Path, offset_gloss: dict[int, str]) -> dict[str, str]:
    lemma_def: dict[str, str] = {}
    for name in INDEX_FILES:
        path = wn_dir / name
        if not path.exists():
            continue
        with path.open(encoding="ascii", errors="ignore") as f:
            for line in f:
                if not line.strip() or line.startswith(" "):
                    continue
                fields = line.split()
                if len(fields) < 4:
                    continue
                lemma = fields[0].replace("_", " ").lower()
                try:
                    synset_cnt = int(fields[2])
                    off = int(fields[-synset_cnt])
                except ValueError:
                    continue
                gloss = offset_gloss.get(off)
                if gloss and lemma not in lemma_def:
                    lemma_def[lemma] = gloss
    return lemma_def


def parse_noun_exceptions(wn_dir: Path) -> dict[str, str]:
    """Plural surface forms use ``noun.exc`` only (not verb/adj/adv)."""
    exc: dict[str, str] = {}
    path = wn_dir / "noun.exc"
    if not path.exists():
        return exc
    with path.open(encoding="ascii", errors="ignore") as f:
        for line in f:
            parts = line.split()
            if len(parts) < 2:
                continue
            base = parts[0].replace("_", " ").lower()
            for form in parts[1:]:
                exc[form.replace("_", " ").lower()] = base
    return exc


def parse_exceptions(wn_dir: Path) -> dict[str, str]:
    exc: dict[str, str] = {}
    for name in EXC_FILES:
        path = wn_dir / name
        if not path.exists():
            continue
        with path.open(encoding="ascii", errors="ignore") as f:
            for line in f:
                parts = line.split()
                if len(parts) < 2:
                    continue
                base = parts[0].replace("_", " ").lower()
                for form in parts[1:]:
                    exc[form.replace("_", " ").lower()] = base
    return exc


def lookup_gloss(word: str, lemma_def: dict[str, str], exc: dict[str, str]) -> str | None:
    w = word.lower()
    if w in lemma_def:
        return lemma_def[w]
    base = exc.get(w)
    if base and base in lemma_def:
        return lemma_def[base]
    if w.endswith("s") and w[:-1] in lemma_def:
        return lemma_def[w[:-1]]
    if w.endswith("es") and w[:-2] in lemma_def:
        return lemma_def[w[:-2]]
    if w.endswith("ed") and w[:-2] in lemma_def:
        return lemma_def[w[:-2]]
    if w.endswith("ing") and w[:-3] in lemma_def:
        return lemma_def[w[:-3]]
    return None


def load_allowlist(path: Path) -> set[str]:
    if not path.exists():
        raise SystemExit(f"Missing {path} (Google 10k common words list)")
    return {
        line.strip().lower()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    }


def iter_system_words(min_len: int, max_len: int) -> set[str]:
    if not SYSTEM_DICT.exists():
        raise SystemExit(f"Missing {SYSTEM_DICT}")
    words: set[str] = set()
    with SYSTEM_DICT.open(errors="ignore") as f:
        for line in f:
            w = line.strip().lower()
            if w.isalpha() and min_len <= len(w) <= max_len:
                words.add(w)
    return words


def lemma_in_range(lemma: str, min_len: int, max_len: int) -> bool:
    if " " in lemma:
        return False
    if not lemma.isalpha():
        return False
    return min_len <= len(lemma) <= max_len


def build_common(
    repo: Path,
    allowlist_path: Path,
    min_len: int,
    max_len: int,
    wn_dir: Path,
) -> tuple[list[dict[str, str]], dict[str, int]]:
    allowlist = load_allowlist(allowlist_path)
    offset_gloss = parse_glosses(wn_dir)
    lemma_def = parse_lemma_index(wn_dir, offset_gloss)
    exc = parse_exceptions(wn_dir)

    candidates = iter_system_words(min_len, max_len) & allowlist
    out: list[dict[str, str]] = []
    skipped_no_gloss = 0
    for word in sorted(candidates):
        defn = lookup_gloss(word, lemma_def, exc)
        if not defn:
            skipped_no_gloss += 1
            continue
        out.append({"word": word, "definition": defn[:280]})

    all_dict = iter_system_words(min_len, max_len)
    skipped_not_common = sum(1 for word in all_dict if word not in allowlist)

    stats = {
        "allowlist_size": len(allowlist),
        "skipped_no_gloss": skipped_no_gloss,
        "skipped_not_common": skipped_not_common,
    }
    return out, stats


def inflection_surface_forms(base: str, exc: dict[str, str]) -> list[str]:
    """Regular noun plurals + WordNet ``*.exc`` lines (no blind ``+es`` on every lemma)."""
    forms: list[str] = []
    for form, parent in exc.items():
        if parent == base:
            forms.append(form)
    if base.endswith(("ch", "sh")):
        forms.append(base + "es")
    elif base.endswith(("s", "x", "z")):
        forms.append(base + "es")
    elif (
        base.endswith("y")
        and len(base) > 1
        and base[-2] not in "aeiou"
    ):
        forms.append(base[:-1] + "ies")
    elif base.endswith("fe"):
        forms.append(base[:-2] + "ves")
    elif base.endswith("f"):
        forms.append(base[:-1] + "ves")
    else:
        forms.append(base + "s")
    seen: set[str] = set()
    deduped: list[str] = []
    for f in forms:
        if f not in seen:
            seen.add(f)
            deduped.append(f)
    return deduped


def expand_inflected_forms(
    entries: list[dict[str, object]],
    lemma_def: dict[str, str],
    exc: dict[str, str],
    noun_exc: dict[str, str],
    noun_by_lemma: dict[str, list[str]],
    noun_offsets: dict[str, list[int]],
    synset_lex: dict[int, int],
    synset_words: dict[int, list[str]],
    min_len: int,
    max_len: int,
) -> tuple[list[dict[str, object]], int]:
    """Add **noun** plural surface forms only (no ``dashing`` → ``dashings``, etc.)."""
    existing = {e["word"] for e in entries}
    out = list(entries)
    added = 0
    for base in sorted(noun_by_lemma):
        if " " in base or not base.isalpha():
            continue
        if not (min_len <= len(base) <= max_len):
            continue
        if skip_noun_plural(base, noun_offsets.get(base, []), synset_lex, synset_words):
            continue
        candidates = inflection_surface_forms(base, noun_exc)
        for form in candidates:
            if form in existing:
                continue
            if " " in form or not form.isalpha():
                continue
            if not (min_len <= len(form) <= max_len):
                continue
            defn = lookup_gloss(form, lemma_def, exc)
            if not defn:
                continue
            row: dict[str, object] = {"word": form, "definition": defn[:280]}
            attach_noun_senses(row, noun_by_lemma, exc)
            out.append(row)
            existing.add(form)
            added += 1
    out.sort(key=lambda e: e["word"])
    return out, added


def build_wordnet_full(
    wn_dir: Path,
    min_len: int,
    max_len: int,
) -> tuple[list[dict[str, str]], dict[str, int]]:
    offset_gloss = parse_glosses(wn_dir)
    lemma_def = parse_lemma_index(wn_dir, offset_gloss)
    noun_by_lemma, noun_offsets = parse_noun_lemma_index(wn_dir, offset_gloss)
    exc = parse_exceptions(wn_dir)

    out: list[dict[str, object]] = []
    skipped_range = 0
    skipped_multi = 0
    for lemma, defn in sorted(lemma_def.items()):
        if " " in lemma:
            skipped_multi += 1
            continue
        if not lemma.isalpha():
            skipped_multi += 1
            continue
        if not (min_len <= len(lemma) <= max_len):
            skipped_range += 1
            continue
        row: dict[str, object] = {"word": lemma, "definition": defn[:280]}
        attach_noun_senses(row, noun_by_lemma, exc)
        out.append(row)

    noun_exc = parse_noun_exceptions(wn_dir)
    synset_lex, synset_words = parse_noun_synset_meta(wn_dir)
    out, inflected_added = expand_inflected_forms(
        out,
        lemma_def,
        exc,
        noun_exc,
        noun_by_lemma,
        noun_offsets,
        synset_lex,
        synset_words,
        min_len,
        max_len,
    )
    for row in out:
        polish_entry_glosses(row, lemma_def)

    stats = {
        "lemma_index_size": len(lemma_def),
        "skipped_range": skipped_range,
        "skipped_multi_or_nonalpha": skipped_multi,
        "inflected_added": inflected_added,
    }
    return out, stats


def write_words(path: Path, entries: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(entries, separators=(",", ":")), encoding="utf-8")


def write_manifest(
    manifest_path: Path,
    *,
    output_path: Path,
    mode: str,
    min_len: int,
    max_len: int,
    entry_count: int,
    wn_dir: Path,
) -> None:
    payload = output_path.read_bytes()
    manifest = {
        "source_name": "wordnet",
        "wn_dir": str(wn_dir),
        "mode": mode,
        "min_len": min_len,
        "max_len": max_len,
        "entry_count": entry_count,
        "built_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "output_path": str(output_path),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    repo = Path(__file__).resolve().parent.parent
    p = argparse.ArgumentParser(description="Build glossed words.json from WordNet.")
    p.add_argument(
        "--mode",
        choices=("common", "wordnet-full"),
        default="common",
        help="common: system dict ∩ allowlist (legacy); wordnet-full: all lemmas in range",
    )
    p.add_argument("--min-len", type=int, default=4, metavar="N")
    p.add_argument("--max-len", type=int, default=10, metavar="N")
    p.add_argument(
        "--output",
        type=Path,
        default=repo / "data" / "words.json",
        help="Output JSON path (default: data/words.json)",
    )
    p.add_argument(
        "--emit-manifest",
        type=Path,
        default=None,
        metavar="PATH",
        help="Write build metadata JSON (counts, sha256)",
    )
    p.add_argument(
        "--allowlist",
        type=Path,
        default=repo / "data" / "common_allowlist.txt",
        help="Allowlist file (common mode only; optional soft filter in wordnet-full)",
    )
    return p.parse_args(argv)


def test_inflection_surface_forms() -> None:
    """Regression: no blind ``lemma+es`` surface forms (balkeres, orchides, …)."""
    exc: dict[str, str] = {}
    balk = inflection_surface_forms("balker", exc)
    assert "balkers" in balk and "balkeres" not in balk
    orch = inflection_surface_forms("orchid", exc)
    assert "orchids" in orch and "orchides" not in orch


def test_noun_only_plural_bases() -> None:
    """Adjectives in lemma index must not receive generated plurals."""
    wn_dir = resolve_wn_dir()
    if not wordnet_dict_ready(wn_dir):
        return
    offset_gloss = parse_glosses(wn_dir)
    noun_by_lemma, _ = parse_noun_lemma_index(wn_dir, offset_gloss)
    assert "dashing" not in noun_by_lemma
    assert "balker" in noun_by_lemma


def test_cross_reference_gloss_expansion() -> None:
    lemma_def = {
        "smyrnium": "Alexanders",
        "alexanders": (
            "European herb somewhat resembling celery widely naturalized "
            "in Britain coastal regions and often cultivated as a potherb"
        ),
    }
    short = expand_cross_reference_gloss("smyrnium", "Alexanders", lemma_def)
    assert "celery" in short.lower()
    assert short != "Alexanders"


def test_proper_noun_plural_skip() -> None:
    wn_dir = resolve_wn_dir()
    if not wordnet_dict_ready(wn_dir):
        return
    offset_gloss = parse_glosses(wn_dir)
    _, noun_offsets = parse_noun_lemma_index(wn_dir, offset_gloss)
    synset_lex, synset_words = parse_noun_synset_meta(wn_dir)
    assert skip_noun_plural("baghdad", noun_offsets["baghdad"], synset_lex, synset_words)
    assert not skip_noun_plural("balker", noun_offsets["balker"], synset_lex, synset_words)


def main(argv: list[str] | None = None) -> int:
    test_inflection_surface_forms()
    test_cross_reference_gloss_expansion()
    test_noun_only_plural_bases()
    test_proper_noun_plural_skip()
    args = parse_args(argv)
    wn_dir = resolve_wn_dir()
    if not wordnet_dict_ready(wn_dir):
        print("WordNet dict not found or incomplete at", wn_dir, file=sys.stderr)
        missing = missing_wordnet_files(wn_dir)
        if missing:
            print("Missing:", ", ".join(missing), file=sys.stderr)
        print(
            "Unpack wn3.1.dict.tar.gz so dict/ contains data.noun, index.noun, …",
            file=sys.stderr,
        )
        print(
            "  curl -LO https://wordnetcode.princeton.edu/wn3.1.dict.tar.gz",
            file=sys.stderr,
        )
        print(
            "  mkdir -p /tmp/wn && tar -xzf wn3.1.dict.tar.gz -C /tmp/wn",
            file=sys.stderr,
        )
        print("Or set WN_DIR to your dict/ directory.", file=sys.stderr)
        return 1
    if args.min_len > args.max_len:
        print("--min-len must be <= --max-len", file=sys.stderr)
        return 1

    if args.mode == "common":
        out, stats = build_common(
            Path(__file__).resolve().parent.parent,
            args.allowlist,
            args.min_len,
            args.max_len,
            wn_dir,
        )
        write_words(args.output, out)
        print(f"Mode: common (allowlist ∩ {SYSTEM_DICT})")
        print(f"Allowlist size: {stats['allowlist_size']}")
        print(f"Wrote {len(out)} entries (WordNet + common filter)")
        print(f"Skipped {stats['skipped_no_gloss']} common words without WordNet gloss")
        print(f"Excluded ~{stats['skipped_not_common']} system words not on allowlist")
    else:
        out, stats = build_wordnet_full(wn_dir, args.min_len, args.max_len)
        write_words(args.output, out)
        print("Mode: wordnet-full (all glossed lemmas in length range)")
        print(f"Lemma index size: {stats['lemma_index_size']}")
        print(f"Wrote {len(out)} entries")
        print(f"Skipped {stats['skipped_range']} lemmas outside length range")
        print(f"Skipped {stats['skipped_multi_or_nonalpha']} multi-token / non-alpha lemmas")
        print(f"Added {stats['inflected_added']} inflected surface forms")

    print(f"Output: {args.output}")
    print(f"Size: {args.output.stat().st_size / 1_048_576:.2f} MiB")

    if args.emit_manifest:
        write_manifest(
            args.emit_manifest,
            output_path=args.output,
            mode=args.mode,
            min_len=args.min_len,
            max_len=args.max_len,
            entry_count=len(out),
            wn_dir=wn_dir,
        )
        print(f"Manifest: {args.emit_manifest}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())