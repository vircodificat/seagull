"""
    Cross-reference CMU dictionary words against steno dictionaries.

    Produces build/cmucrossref.json with the structure:

        {
            "hello": {
                "pronunciations": [["HH", "AH0", "L", "OW1"], ...],
                "outlines": {
                    "main.json": ["HEL/HRO", ...],
                    "lapwing-base.json": ["H-L", ...],
                    "seagull_base.json": []
                }
            },
            ...
        }

"""

import json
import os
from collections import defaultdict


CMUDICT_PATH = "data/cmudict/cmudict"
STENO_DICTS = ["main.json", "lapwing-base.json", "seagull_base.json"]
DATA_DIR = "data"
BUILD_DIR = "build"
OUTPUT_PATH = os.path.join(BUILD_DIR, "cmucrossref.json")


def parse_cmudict(path):
    """Parse the CMU pronouncing dictionary.

    Returns a dict mapping lowercase word -> list of phoneme lists.
    Entries with the same word but different numbers are multiple pronunciations.
    """
    pronunciations = defaultdict(list)
    with open(path, encoding="latin-1") as f:
        for line in f:
            line = line.strip()
            # Skip comments / blank lines
            if not line or line.startswith(";;;"):
                continue
            parts = line.split()
            if len(parts) < 2:
                continue
            # parts[0] = WORD or WORD(N), parts[1] = pronunciation index, parts[2:] = phonemes
            # Format: WORD N PHONEME ...
            word_field = parts[0]
            # Strip trailing pronunciation index in parentheses, e.g. WORD(2) -> WORD
            if word_field.endswith(")") and "(" in word_field:
                word_field = word_field[: word_field.index("(")]
            word = word_field.lower()
            # parts[1] is the numeric index; phonemes start at parts[2]
            phonemes = parts[2:]
            if phonemes:
                pronunciations[word].append(phonemes)
    return dict(pronunciations)


def load_steno_dict(filename):
    """Load a steno JSON dictionary and return a word -> [outline, ...] mapping."""
    path = os.path.join(DATA_DIR, filename)
    word_to_outlines = defaultdict(list)
    with open(path, encoding="utf-8") as f:
        steno = json.load(f)
    for outline, word in steno.items():
        # Normalise: strip steno formatting artifacts like {^} etc. is not needed;
        # we match the raw translation string case-insensitively.
        word_to_outlines[word.lower()].append(outline)
    return dict(word_to_outlines)


def main():
    os.makedirs(BUILD_DIR, exist_ok=True)

    print("Parsing CMU dictionary…")
    cmu = parse_cmudict(CMUDICT_PATH)
    print(f"  {len(cmu)} unique words found.")

    print("Loading steno dictionaries…")
    steno_indexes = {}
    for filename in STENO_DICTS:
        steno_indexes[filename] = load_steno_dict(filename)
        print(f"  {filename}: {len(steno_indexes[filename])} unique translations.")

    print("Building cross-reference…")
    result = {}
    for word, prons in sorted(cmu.items()):
        outlines = {}
        for filename in STENO_DICTS:
            outlines[filename] = sorted(steno_indexes[filename].get(word, []))
        if not any(outlines.values()):
            continue
        result[word] = {
            "pronunciations": prons,
            "outlines": outlines,
        }

    print(f"Writing {OUTPUT_PATH}…")
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print("Done.")


if __name__ == "__main__":
    main()
