"""Group inflected words from build/base_vocab_analysis.json by their base form.

Produces two files:

build/regular_vocab.json
    Base words as keys; values map each inflection type to the (regular)
    inflected form.  Words with irregular=True are excluded entirely.
    Example:
        { "work": { "third_person_singular": "works", "past": "worked", ... } }

build/irregular_vocab.json
    Base words (lemmas) of irregular inflected forms as keys; values map each
    inflection type to a list of irregular forms with that type.
    (The base word itself carries irregular=None — that is expected.)
    Example:
        { "go": { "inflected": ["went"] }, "be": { "inflected": ["am", "are", "was", "were", "been"] } }
"""

import json
import os
from collections import defaultdict

INPUT_PATH   = os.path.join("build", "base_vocab_analysis.json")
REGULAR_PATH = os.path.join("build", "regular_vocab.json")
IRREGULAR_PATH = os.path.join("build", "irregular_vocab.json")


def main():
    data: dict = json.load(open(INPUT_PATH, encoding="utf-8"))
    print(f"Loaded {len(data)} entries from {INPUT_PATH}")

    # ------------------------------------------------------------------
    # Pass 1 – bucket every non-base word by lemma and regularity.
    # ------------------------------------------------------------------
    regular_forms:   dict[str, dict[str, str]]       = defaultdict(dict)
    irregular_forms: dict[str, dict[str, list[str]]] = defaultdict(lambda: defaultdict(list))

    for word, entry in data.items():
        if entry["inflection"] == "base":
            continue  # base forms become keys, not values

        lemma     = entry["lemma"]
        inflection = entry["inflection"]
        irregular  = entry["irregular"]

        if irregular is False:
            # Regular: one canonical form per inflection type is expected.
            regular_forms[lemma][inflection] = word
        elif irregular is True:
            irregular_forms[lemma][inflection].append(word)

    # ------------------------------------------------------------------
    # Pass 2 – collect base-form words that act as keys.
    # ------------------------------------------------------------------
    base_words: set[str] = {
        word for word, entry in data.items()
        if entry["inflection"] == "base"
    }

    # ------------------------------------------------------------------
    # Build regular_vocab: base words whose inflected forms are all regular.
    # Irregular=True words are excluded, so only lemmas that appear in
    # regular_forms (and whose base form is in the vocab) are included.
    # ------------------------------------------------------------------
    regular_vocab = {
        lemma: regular_forms[lemma]
        for lemma in sorted(regular_forms)
        if lemma in base_words and regular_forms[lemma]
    }

    # ------------------------------------------------------------------
    # Build irregular_vocab: lemmas that have at least one irregular form.
    # Convert inner defaultdicts to plain dicts for JSON serialisation.
    # ------------------------------------------------------------------
    irregular_vocab = {
        lemma: dict(irregular_forms[lemma])
        for lemma in sorted(irregular_forms)
        if lemma in base_words and irregular_forms[lemma]
    }

    # ------------------------------------------------------------------
    # Write outputs.
    # ------------------------------------------------------------------
    os.makedirs("build", exist_ok=True)

    with open(REGULAR_PATH, "w", encoding="utf-8") as f:
        json.dump(regular_vocab, f, ensure_ascii=False, indent=2)
    print(f"Written {len(regular_vocab)} entries to {REGULAR_PATH}")

    with open(IRREGULAR_PATH, "w", encoding="utf-8") as f:
        json.dump(irregular_vocab, f, ensure_ascii=False, indent=2)
    print(f"Written {len(irregular_vocab)} entries to {IRREGULAR_PATH}")


if __name__ == "__main__":
    main()
