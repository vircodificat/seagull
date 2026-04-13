"""Analyse every word in data/base_vocab.json with NLTK and write
build/base_vocab_analysis.json.

Each entry looks like:
{
    "girls": {
        "lemma":      "girl",
        "pos":        ["noun"],
        "inflection": "plural",
        "is_lemma":   false
    },
    ...
}

Fields
------
lemma      : base / dictionary form of the word
pos        : parts of speech according to WordNet (may be empty for
             function words / contractions / proper nouns not in WN)
inflection : morphological form relative to the lemma, one of:
               base | plural | possessive | third_person_singular |
               past | past_participle | present_participle |
               comparative | superlative | contraction | inflected
is_lemma   : True when the word is already its own base form
irregular  : True  — inflection does not follow standard English rules
             False — inflection follows standard English rules
             None  — word is a base form (inflection == "base")
"""

import json
import os
import re

import nltk
from nltk.corpus import wordnet as wn
from nltk.stem import WordNetLemmatizer

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

DATA_DIR  = "data"
BUILD_DIR = "build"
VOCAB_PATH  = os.path.join(DATA_DIR,  "base_vocab.json")
OUTPUT_PATH = os.path.join(BUILD_DIR, "base_vocab_analysis.json")

WN_POS_STR = {
    wn.NOUN:    "noun",
    wn.VERB:    "verb",
    wn.ADJ:     "adjective",
    wn.ADJ_SAT: "adjective",
    wn.ADV:     "adverb",
}
STR_WN_POS = {
    "noun":      wn.NOUN,
    "verb":      wn.VERB,
    "adjective": wn.ADJ,
    "adverb":    wn.ADV,
}
WN_POS_ORDER = [wn.NOUN, wn.VERB, wn.ADJ, wn.ADV]

CONTRACTION_RE = re.compile(
    r"(?:'t|'re|'ve|'ll|'d|'m|n't|'bout|'round|'cause|'em)$",
    re.IGNORECASE,
)

# Pronouns / function words whose "'s" form is a contraction ("is"/"has"/"us"),
# not a possessive.  Everything else (nouns, indefinite pronouns, …) defaults
# to possessive.
APOSTROPHE_S_CONTRACTIONS = {
    "he", "she", "it", "that", "this", "who", "what",
    "where", "when", "how", "there", "here", "let",
}

lemmatizer = WordNetLemmatizer()


def _load_wordnet_exceptions() -> set[str]:
    """Return the set of all irregular inflected forms from WordNet's .exc files."""
    irregular: set[str] = set()
    for pos in ["noun", "verb", "adj", "adv"]:
        try:
            raw = nltk.data.load(f"corpora/wordnet/{pos}.exc", format="raw").decode()
        except Exception:
            continue
        for line in raw.splitlines():
            parts = line.strip().split()
            if parts:
                irregular.add(parts[0])
    return irregular


# Built once at import time so the main loop pays no I/O cost per word.
IRREGULAR_FORMS: set[str] = _load_wordnet_exceptions()


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def wordnet_pos(word: str) -> list[str]:
    """Return deduplicated POS labels from WordNet synsets."""
    seen, result = set(), []
    for s in wn.synsets(word):
        label = WN_POS_STR.get(s.pos())
        if label and label not in seen:
            seen.add(label)
            result.append(label)
    return result


def best_lemma(word: str, pos_list: list[str]) -> str:
    """Return the base form, trying known POS first then all WN POS."""
    # Prefer a POS that actually changes the word (i.e. it's inflected)
    for pos_str in pos_list:
        wn_pos = STR_WN_POS.get(pos_str)
        if wn_pos:
            lem = lemmatizer.lemmatize(word, wn_pos)
            if lem != word:
                return lem
    # Fallback: try every WN POS
    for wn_pos in WN_POS_ORDER:
        lem = lemmatizer.lemmatize(word, wn_pos)
        if lem != word:
            return lem
    return word


def detect_inflection(word: str, lemma: str, pos_list: list[str]) -> str:
    """Classify the morphological relationship between *word* and *lemma*."""
    # Plural possessive: cats'
    if word.endswith("s'"):
        return "possessive"

    # Words that form 's contractions (= "is" / "has" / "us"), not possessives.
    if word.endswith("'s") and word[:-2].lower() in APOSTROPHE_S_CONTRACTIONS:
        return "contraction"

    # Everything else ending in 's is a possessive: accident's, anyone's, …
    if word.endswith("'s"):
        return "possessive"

    # Remaining contractions: don't, she'll, I'm, 'bout, …
    if CONTRACTION_RE.search(word):
        return "contraction"

    if word == lemma:
        return "base"

    # Verb inflections take priority when the word is known as a verb
    if "verb" in pos_list:
        if word.endswith("ing"):
            return "present_participle"
        if word.endswith("ed"):
            return "past"
        if word.endswith("s"):
            return "third_person_singular"

    # Noun plurality
    if "noun" in pos_list and word.endswith("s"):
        return "plural"

    # Adjective / adverb comparison
    if "adjective" in pos_list or "adverb" in pos_list:
        if word.endswith("est"):
            return "superlative"
        if word.endswith("er"):
            return "comparative"

    return "inflected"


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    os.makedirs(BUILD_DIR, exist_ok=True)

    words: list[str] = json.load(open(VOCAB_PATH, encoding="utf-8"))
    print(f"Loaded {len(words)} words from {VOCAB_PATH}")

    result = {}
    for word in words:
        pos   = wordnet_pos(word)
        lemma = best_lemma(word, pos)
        infl  = detect_inflection(word, lemma, pos)
        if infl == "base":
            irregular = None
        else:
            irregular = word in IRREGULAR_FORMS

        result[word] = {
            "lemma":      lemma,
            "pos":        pos,
            "inflection": infl,
            "is_lemma":   word == lemma,
            "irregular":  irregular,
        }

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"Written {len(result)} entries to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
