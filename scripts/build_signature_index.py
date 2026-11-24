#!/usr/bin/env python3
"""Build data/signature_index.json from words.json (SPEC_SIGNATURE_INDEX §4–5)."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

DEFINITION_MAX_LEN = 280
DEFAULT_MIN_LEN = 4
DEFAULT_MAX_LEN = 10


def signature_key(word: str) -> str:
    """Appendix B — sorted lowercase a–z letters."""
    chars = [c for c in word.casefold() if c.isascii() and c.isalpha()]
    chars.sort()
    return "".join(chars)


def is_valid_word(word: str) -> bool:
    return bool(word) and word.isascii() and word.isalpha()


def has_definition(defn: object) -> bool:
    return isinstance(defn, str) and bool(defn.strip())


def truncate_definition(defn: str) -> str:
    return defn[:DEFINITION_MAX_LEN]


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def deterministic_built_at(source_sha256: str) -> str:
    """Stable ISO-8601 UTC from lexicon hash (SPEC build_deterministic)."""
    if epoch_env := __import__("os").environ.get("SOURCE_DATE_EPOCH"):
        epoch = int(epoch_env)
    else:
        epoch = int(source_sha256[:8], 16) % 2_000_000_000
    return datetime.fromtimestamp(epoch, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def source_path_label(path: Path) -> str:
    try:
        return str(path.relative_to(Path.cwd()))
    except ValueError:
        return str(path)


def load_entries(
    path: Path, min_len: int, max_len: int
) -> tuple[list[dict[str, str]], dict[str, int]]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        raise SystemExit(f"{path}: expected JSON array")

    stats = {"skipped_invalid": 0, "skipped_no_definition": 0, "skipped_length": 0, "deduped": 0}
    by_word: dict[str, dict[str, str]] = {}

    for row in raw:
        if not isinstance(row, dict):
            stats["skipped_invalid"] += 1
            continue
        word_raw = row.get("word")
        if not isinstance(word_raw, str) or not is_valid_word(word_raw):
            stats["skipped_invalid"] += 1
            continue
        word = word_raw.casefold()
        if not (min_len <= len(word) <= max_len):
            stats["skipped_length"] += 1
            continue
        defn = row.get("definition")
        if not has_definition(defn):
            stats["skipped_no_definition"] += 1
            continue
        entry: dict[str, object] = {
            "word": word,
            "definition": truncate_definition(defn.strip()),
        }
        senses = row.get("noun_senses")
        if isinstance(senses, list):
            glosses = [
                truncate_definition(s.strip())
                for s in senses
                if isinstance(s, str) and s.strip()
            ]
            if glosses:
                entry["noun_senses"] = glosses
        if word in by_word:
            stats["deduped"] += 1
            continue
        by_word[word] = entry

    entries = sorted(by_word.values(), key=lambda e: e["word"])
    return entries, stats


def build_buckets(entries: list[dict[str, str]]) -> dict[str, list[dict[str, str]]]:
    buckets: dict[str, list[dict[str, str]]] = defaultdict(list)
    for entry in entries:
        key = signature_key(entry["word"])
        buckets[key].append(entry)
    for key in buckets:
        buckets[key].sort(key=lambda e: e["word"])
    return dict(sorted(buckets.items()))


def build_index_doc(
    *,
    source_path: str,
    source_sha256: str,
    min_size: int,
    max_size: int,
    buckets: dict[str, list[dict[str, str]]],
    entry_count: int,
) -> dict:
    bucket_count = len(buckets)
    return {
        "format_version": 1,
        "source": {"path": source_path, "sha256": source_sha256},
        "locale": "en-US",
        "min_size": min_size,
        "max_size": max_size,
        "built_at": deterministic_built_at(source_sha256),
        "entry_count": entry_count,
        "bucket_count": bucket_count,
        "buckets": buckets,
    }


def serialize_index(doc: dict) -> str:
    return json.dumps(doc, indent=2, ensure_ascii=False) + "\n"


def bucket_stats(buckets: dict[str, list[dict[str, str]]]) -> tuple[int, int]:
    if not buckets:
        return 0, 0
    sizes = [len(v) for v in buckets.values()]
    max_bucket = max(sizes)
    ambiguous = sum(1 for s in sizes if s > 1)
    return max_bucket, ambiguous


def main() -> None:
    parser = argparse.ArgumentParser(description="Build signature_index.json from words JSON array.")
    parser.add_argument("--input", type=Path, help="Source lexicon JSON (array of {word, definition})")
    parser.add_argument("--output", type=Path, help="Output signature_index.json path")
    parser.add_argument("--min-len", type=int, default=DEFAULT_MIN_LEN, dest="min_len")
    parser.add_argument("--max-len", type=int, default=DEFAULT_MAX_LEN, dest="max_len")
    parser.add_argument(
        "--print-key",
        metavar="WORD",
        help="Print signature_string for WORD and exit (Appendix B)",
    )
    args = parser.parse_args()

    if args.print_key is not None:
        word = args.print_key
        if not is_valid_word(word):
            print(f"invalid word: {word!r}", file=sys.stderr)
            raise SystemExit(2)
        print(signature_key(word))
        if args.input is None:
            return

    if args.input is None or args.output is None:
        parser.error("--input and --output are required unless only --print-key is used")

    if args.min_len > args.max_len:
        raise SystemExit("--min-len must be <= --max-len")

    input_path = args.input.resolve()
    if not input_path.is_file():
        raise SystemExit(f"input not found: {input_path}")

    source_sha = sha256_file(input_path)
    entries, load_stats = load_entries(input_path, args.min_len, args.max_len)
    buckets = build_buckets(entries)
    entry_count = len(entries)
    max_bucket_size, ambiguous_buckets = bucket_stats(buckets)

    doc = build_index_doc(
        source_path=source_path_label(input_path),
        source_sha256=source_sha,
        min_size=args.min_len,
        max_size=args.max_len,
        buckets=buckets,
        entry_count=entry_count,
    )

    out_text = serialize_index(doc)
    doc["index_bytes"] = len(out_text.encode("utf-8"))
    out_text = serialize_index(doc)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(out_text, encoding="utf-8")

    out_sha = hashlib.sha256(out_text.encode("utf-8")).hexdigest()
    print(f"entry_count={entry_count}", file=sys.stderr)
    print(f"bucket_count={len(buckets)}", file=sys.stderr)
    print(f"max_bucket_size={max_bucket_size}", file=sys.stderr)
    print(f"ambiguous_buckets={ambiguous_buckets}", file=sys.stderr)
    print(f"output_sha256={out_sha}", file=sys.stderr)
    if any(load_stats.values()):
        print(f"load_stats={load_stats}", file=sys.stderr)


if __name__ == "__main__":
    main()