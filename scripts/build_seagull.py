"""Build seagull.json by starting from BASE_PATH and merging dictionaries
in order.  Outlines already present are never overwritten.

After PRE_INFLECTION_DICTS are merged, inflection rules are applied using
build/regular_vocab.json: for every base word already in the dictionary,
a suffix stroke is appended to its outline(s) to produce outlines for its
regular inflected forms.  POST_INFLECTION_DICTS are merged last.

Output: build/seagull.json
"""

import json
import os
from collections import defaultdict

BASE_PATH          = "data/seagull_base.json"
OUTPUT_PATH        = "build/seagull.json"
REGULAR_VOCAB_PATH = "build/regular_vocab.json"

# Dictionaries merged before inflection rules are applied (highest priority first).
# Outlines already present in an earlier entry are skipped.
PRE_INFLECTION_DICTS = [
    "data/commands.json",
    "data/punctuation.json",
    "build/obvious_outlines.json",
    "build/reasonable_outlines.json",
]

# Dictionaries merged after inflection rules are applied.
POST_INFLECTION_DICTS = [
#    "data/stened.json",
]

# ---------------------------------------------------------------------------
# Inflection suffix rules.
# Each entry is (inflection_type, suffix_stroke).
#   inflection_type  – key used in regular_vocab.json inflection dicts
#   suffix_stroke    – steno stroke appended to the base-word outline
#
# To add a rule:   append a new tuple.
# To remove a rule: delete or comment out the relevant line.
# ---------------------------------------------------------------------------
INFLECTION_RULES: list[tuple[str, str]] = [
    ("plural",                "-S"),   # cats, boxes
    ("third_person_singular", "-S"),   # runs, watches
    ("past",                  "-D"),   # walked, jumped
    ("present_participle",    "-G"),   # running, jumping
    ("comparative",           "-R"),   # faster, bigger
    ("possessive",            "*S"),   # cat's, dog's
]


def merge_dict(result: dict[str, str], path: str) -> None:
    """Merge a steno dictionary file into *result* (no overwrite)."""
    source: dict[str, str] = json.load(open(path, encoding="utf-8"))
    added = 0
    for outline, word in source.items():
        # Skip anything with a digit.
        if any(ch.isdigit() for ch in outline):
            continue

        if '#' in outline:
            continue

        # Outlines starting with "A/" are rewritten to "A*/".
        if outline.startswith("A/"):
            outline = "A*/" + outline[2:]
        if outline not in result:
            result[outline] = word
            added += 1
    print(f"{path}: added {added} of {len(source)}")


def apply_inflection_rules(
    result: dict[str, str],
    regular_vocab: dict[str, dict[str, str]],
) -> None:
    """Append suffix-stroke outlines for regular inflected forms.

    For every base word that already has at least one outline in *result*,
    each matching rule in INFLECTION_RULES produces a new outline:

        base_outline/suffix_stroke  →  inflected_word

    Existing outlines are never overwritten.
    """
    # Invert result: word -> list of outlines currently in the dictionary.
    word_to_outlines: dict[str, list[str]] = defaultdict(list)
    for outline, word in result.items():
        word_to_outlines[word].append(outline)

    added = 0
    for base_word, inflections in regular_vocab.items():
        base_outlines = word_to_outlines.get(base_word)
        if not base_outlines:
            continue  # base word not yet in dictionary; skip

        for inflection_type, suffix_stroke in INFLECTION_RULES:
            inflected_form = inflections.get(inflection_type)
            if inflected_form is None:
                continue

            for base_outline in base_outlines:
                new_outline = f"{base_outline}/{suffix_stroke}"
                if new_outline not in result:
                    result[new_outline] = inflected_form
                    added += 1

    print(f"inflection rules: added {added} entries")


def main():
    result: dict[str, str] = json.load(open(BASE_PATH, encoding="utf-8"))
    print(f"{BASE_PATH}: {len(result)} entries")

    for path in PRE_INFLECTION_DICTS:
        merge_dict(result, path)

    regular_vocab: dict[str, dict[str, str]] = json.load(
        open(REGULAR_VOCAB_PATH, encoding="utf-8")
    )
    print(f"{REGULAR_VOCAB_PATH}: {len(regular_vocab)} base words")
    apply_inflection_rules(result, regular_vocab)

    for path in POST_INFLECTION_DICTS:
        merge_dict(result, path)

    os.makedirs("build", exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"Total: {len(result)} entries written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
