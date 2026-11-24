#!/usr/bin/env python3
"""Rank prefix/suffix n-grams from words.json for workbench pattern hints."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path


def rank_ngrams(
    words: list[str], *, min_len: int, max_len: int, min_word_len: int
) -> tuple[list[str], list[str], Counter[str], Counter[str]]:
    starts = Counter()
    ends = Counter()
    for w in words:
        if not w.isalpha():
            continue
        if len(w) < min_word_len:
            continue
        for n in range(min_len, max_len + 1):
            if len(w) < n:
                continue
            starts[w[:n]] += 1
            ends[w[-n:]] += 1

    def ordered(counter: Counter[str]) -> list[str]:
        ranked = sorted(counter.items(), key=lambda kv: (-kv[1], -len(kv[0]), kv[0]))
        return [g for g, _ in ranked]

    return ordered(starts), ordered(ends), starts, ends


def main() -> None:
    repo = Path(__file__).resolve().parents[1]
    p = argparse.ArgumentParser(description="Build affix pattern lists from words.json.")
    p.add_argument("--input", type=Path, default=repo / "data" / "words.json")
    p.add_argument("--output", type=Path, default=repo / "data" / "affix_patterns.json")
    p.add_argument("--min-len", type=int, default=2)
    p.add_argument("--max-len", type=int, default=5)
    p.add_argument("--min-word-len", type=int, default=3)
    p.add_argument("--top", type=int, default=400, help="Max patterns per side")
    args = p.parse_args()

    raw = json.loads(args.input.read_text())
    words = [e["word"].lower() for e in raw]

    starts, ends, sc, ec = rank_ngrams(
        words,
        min_len=args.min_len,
        max_len=args.max_len,
        min_word_len=args.min_word_len,
    )
    starts = starts[: args.top]
    ends = ends[: args.top]

    payload = {
        "version": 1,
        "source": str(args.input.relative_to(repo)) if args.input.is_relative_to(repo) else str(args.input),
        "word_count": len(words),
        "ngram_len": [args.min_len, args.max_len],
        "top10_starts": [{"gram": g, "count": sc[g]} for g, _ in sc.most_common(10)],
        "top10_ends": [{"gram": g, "count": ec[g]} for g, _ in ec.most_common(10)],
        "prefixes": starts,
        "suffixes": ends,
    }
    args.output.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {args.output} ({len(starts)} prefixes, {len(ends)} suffixes)")


if __name__ == "__main__":
    main()