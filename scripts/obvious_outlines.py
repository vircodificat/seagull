"""Find 'obvious' outlines for base vocabulary words.

A word gets an obvious outline if:
  - it is a key in build/regular_vocab.json, AND
  - it has EXACTLY one outline in data/main.json, AND
  - it has EXACTLY one outline in data/lapwing-base.json, AND
  - those two outlines are identical.

Output: output/obvious_outlines.json  { "OUTLINE": "word", ... }
"""

import json
import os
from collections import defaultdict

REGULAR_VOCAB_PATH  = os.path.join("build", "regular_vocab.json")
MAIN_PATH           = os.path.join("data", "main.json")
LAPWING_PATH        = os.path.join("data", "lapwing-base.json")
OBVIOUS_PATH        = os.path.join("build", "obvious_outlines.json")
REASONABLE_PATH     = os.path.join("build", "reasonable_outlines.json")


def invert(steno_dict: dict[str, str]) -> dict[str, list[str]]:
    """Return word -> [outlines] mapping from a steno outline -> word dict."""
    result: dict[str, list[str]] = defaultdict(list)
    for outline, word in steno_dict.items():
        result[word].append(outline)
    return result


def main():
    base_words: set[str] = set(json.load(open(REGULAR_VOCAB_PATH, encoding="utf-8")))
    print(f"Base words to check: {len(base_words)}")

    main_by_word    = invert(json.load(open(MAIN_PATH,    encoding="utf-8")))
    lapwing_by_word = invert(json.load(open(LAPWING_PATH, encoding="utf-8")))

    obvious:    dict[str, str] = {}
    reasonable: dict[str, str] = {}

    for word in sorted(base_words):
        main_outlines    = set(main_by_word.get(word, []))
        lapwing_outlines = set(lapwing_by_word.get(word, []))

        # Obvious: exactly one outline in each dict and they are the same.
        if len(main_outlines) == 1 and main_outlines == lapwing_outlines:
            (outline,) = main_outlines
            obvious[outline] = word

        # Reasonable: the intersection of the two sets has exactly one element.
        intersection = main_outlines & lapwing_outlines
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
