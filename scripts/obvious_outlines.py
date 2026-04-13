"""Find 'obvious' and 'reasonable' outlines for base vocabulary words.

A word gets an OBVIOUS outline if every dictionary below has EXACTLY one
outline for it and all agree on the same outline.

A word gets a REASONABLE outline if the intersection across all dictionaries
contains EXACTLY one outline.

To add another reference dictionary, append its path to REFERENCE_DICTS.
"""

import json
import os
from collections import defaultdict
from functools import reduce

REGULAR_VOCAB_PATH = os.path.join("build", "regular_vocab.json")
OBVIOUS_PATH       = os.path.join("build", "obvious_outlines.json")
REASONABLE_PATH    = os.path.join("build", "reasonable_outlines.json")

# Reference dictionaries to cross-reference. Add more paths here as needed.
REFERENCE_DICTS = [
    "data/main.json",
    "data/lapwing-base.json",
    "data/stened.json",
]


def invert(steno_dict: dict[str, str]) -> dict[str, set[str]]:
    """Return word -> {outlines} mapping from a steno outline -> word dict."""
    result: dict[str, set[str]] = defaultdict(set)
    for outline, word in steno_dict.items():
        result[word].add(outline)
    return result


def main():
    base_words: set[str] = set(json.load(open(REGULAR_VOCAB_PATH, encoding="utf-8")))
    print(f"Base words to check: {len(base_words)}")

    indexes = [
        invert(json.load(open(path, encoding="utf-8")))
        for path in REFERENCE_DICTS
    ]
    for path, idx in zip(REFERENCE_DICTS, indexes):
        print(f"  {path}: {len(idx)} unique translations")

    obvious:    dict[str, str] = {}
    reasonable: dict[str, str] = {}

    for word in sorted(base_words):
        outlines_per_dict = [idx.get(word, set()) for idx in indexes]

        # Obvious: every dict has exactly one outline and they all agree.
        if all(len(o) == 1 for o in outlines_per_dict):
            if len(set.union(*outlines_per_dict)) == 1:
                (outline,) = outlines_per_dict[0]
                obvious[outline] = word

        # Reasonable: the intersection across all dicts has exactly one outline.
        intersection = reduce(set.intersection, outlines_per_dict)
        if len(intersection) == 1:
            (outline,) = intersection
            reasonable[outline] = word

    os.makedirs("build", exist_ok=True)

    with open(OBVIOUS_PATH, "w", encoding="utf-8") as f:
        json.dump(obvious, f, ensure_ascii=False, indent=2)
    print(f"Written {len(obvious)} obvious outlines to {OBVIOUS_PATH}")

    with open(REASONABLE_PATH, "w", encoding="utf-8") as f:
        json.dump(reasonable, f, ensure_ascii=False, indent=2)
    print(f"Written {len(reasonable)} reasonable outlines to {REASONABLE_PATH}")


if __name__ == "__main__":
    main()
