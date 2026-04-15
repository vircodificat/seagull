"""
Filter build/cmucrossref.json to words whose every pronunciation has
exactly one vowel sound (i.e. one syllable).

CMU phonemes carry a trailing stress digit (0, 1, or 2) on vowels, so
counting phonemes that end with a digit gives the syllable count.

Output: build/cmucrossref_onesyl.json
"""

import json
import os

INPUT_PATH = "build/cmucrossref.json"
OUTPUT_PATH = "build/cmucrossref_onesyl.json"


def syllable_count(phonemes: list[str]) -> int:
    """Return the number of vowel sounds in a phoneme list."""
    return sum(1 for p in phonemes if p[-1].isdigit())


def is_one_syllable(entry: dict) -> bool:
    """Return True if every pronunciation of the entry has exactly one syllable."""
    return all(syllable_count(pron) == 1 for pron in entry["pronunciations"])


def main():
    with open(INPUT_PATH, encoding="utf-8") as f:
        crossref = json.load(f)

    one_syl = {word: entry for word, entry in crossref.items() if is_one_syllable(entry)}

    print(f"Total words: {len(crossref)}")
    print(f"One-syllable words: {len(one_syl)}")

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(one_syl, f, ensure_ascii=False, indent=2)
    print(f"Written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
