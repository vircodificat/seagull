"""Generate random plausible sentences via a trigram Markov chain.

Only words that appear as values in build/seagull.json are used.
The chain is trained on the NLTK Brown corpus and cached in build/markov.json
so that subsequent runs skip the corpus scan entirely.  The cache is
invalidated automatically whenever build/seagull.json is newer than it.

Approach
--------
1. Load the valid vocabulary from seagull.json (alphabetic values only).
2. If build/markov.json exists and is up-to-date, load the chain from it.
3. Otherwise, tokenise every Brown corpus sentence (dropping tokens not in
   the vocab), build a trigram chain and sentence-start list, then write
   the cache.
4. To generate: pick a random start bigram, then at each step sample the
   next word proportionally to its corpus frequency.  Stop when a natural
   length is reached or no continuation exists; retry on short results.

Usage
-----
    uv run scripts/random_sentence.py          # one sentence
    uv run scripts/random_sentence.py 5        # five sentences
"""

import json
import os
import random
import re
import sys

import nltk
from nltk.corpus import brown

SEAGULL_PATH = "build/seagull.json"
CACHE_PATH   = "build/markov.json"
MIN_LEN   = 6    # minimum words in an output sentence
MAX_LEN   = 20   # maximum words in an output sentence
MAX_TRIES = 200  # retries before giving up on a single sentence

# Chain maps a bigram context to a {next_word: count} frequency dict.
Chain = dict[tuple[str, str], dict[str, int]]


def load_valid_words(path: str) -> set[str]:
    """Return lowercase alphabetic words found in seagull.json values."""
    data = json.load(open(path, encoding="utf-8"))
    return {w.lower() for w in data.values() if re.fullmatch(r"[a-z]+", w.lower())}


def build_chain(valid_words: set[str]) -> tuple[Chain, list[tuple[str, str]]]:
    """Build a trigram chain and a list of sentence-starting bigrams."""
    nltk.download("brown", quiet=True)

    chain: Chain = {}
    starts: list[tuple[str, str]] = []

    for sent in brown.sents():
        # Filter to valid vocab only; discard the rest of each token.
        tokens = [
            w.lower() for w in sent
            if re.fullmatch(r"[a-z]+", w.lower()) and w.lower() in valid_words
        ]
        if len(tokens) < 3:
            continue

        starts.append((tokens[0], tokens[1]))

        for i in range(len(tokens) - 2):
            context = (tokens[i], tokens[i + 1])
            freq = chain.setdefault(context, {})
            freq[tokens[i + 2]] = freq.get(tokens[i + 2], 0) + 1

    return chain, starts


# ---------------------------------------------------------------------------
# Cache helpers
# ---------------------------------------------------------------------------

def _chain_to_json(chain: Chain, starts: list[tuple[str, str]]) -> dict:
    """Serialize chain and starts to a JSON-compatible structure.

    Tuple keys are encoded as tab-separated strings (words are alpha-only
    so tabs never appear naturally).
    """
    return {
        "starts": starts,  # list of [w1, w2]; JSON arrays round-trip fine
        "chain": {
            f"{w1}\t{w2}": freq
            for (w1, w2), freq in chain.items()
        },
    }


def _chain_from_json(data: dict) -> tuple[Chain, list[tuple[str, str]]]:
    """Deserialize chain and starts produced by _chain_to_json."""
    starts = [tuple(pair) for pair in data["starts"]]
    chain: Chain = {
        tuple(key.split("\t")): freq          # type: ignore[misc]
        for key, freq in data["chain"].items()
    }
    return chain, starts


def load_cache(seagull_path: str) -> tuple[Chain, list[tuple[str, str]]] | None:
    """Return the cached chain if it exists and is newer than seagull.json.

    Returns None if the cache is missing or stale.
    """
    if not os.path.exists(CACHE_PATH):
        return None
    if os.path.getmtime(seagull_path) > os.path.getmtime(CACHE_PATH):
        return None  # seagull.json was rebuilt; cache is stale
    data = json.load(open(CACHE_PATH, encoding="utf-8"))
    return _chain_from_json(data)


def save_cache(chain: Chain, starts: list[tuple[str, str]]) -> None:
    """Write chain and starts to CACHE_PATH."""
    os.makedirs("build", exist_ok=True)
    with open(CACHE_PATH, "w", encoding="utf-8") as f:
        json.dump(_chain_to_json(chain, starts), f, ensure_ascii=False)
    print(f"Cache written to {CACHE_PATH}", file=sys.stderr)


# ---------------------------------------------------------------------------
# Generation
# ---------------------------------------------------------------------------

def generate_sentence(chain: Chain, starts: list[tuple[str, str]]) -> str:
    """Return a single generated sentence, retrying if it is too short."""
    for _ in range(MAX_TRIES):
        context = random.choice(starts)
        words = list(context)

        for _ in range(MAX_LEN - 2):
            if context not in chain:
                break
            freq = chain[context]
            next_word = random.choices(
                population=list(freq.keys()),
                weights=list(freq.values()),
            )[0]
            words.append(next_word)
            context = (context[1], next_word)

        if len(words) >= MIN_LEN:
            return " ".join(words)

    # Fallback: return whatever the last attempt produced.
    return " ".join(words)


def main() -> None:
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 1

    cached = load_cache(SEAGULL_PATH)
    if cached is not None:
        chain, starts = cached
    else:
        valid_words = load_valid_words(SEAGULL_PATH)
        print(f"Valid vocabulary: {len(valid_words)} words", file=sys.stderr)
        print("Building Markov chain from Brown corpus...", file=sys.stderr)
        chain, starts = build_chain(valid_words)
        save_cache(chain, starts)
        print(
            f"Chain: {len(chain)} contexts, {len(starts)} sentence starts",
            file=sys.stderr,
        )

    for _ in range(count):
        print(generate_sentence(chain, starts))


if __name__ == "__main__":
    main()
