"""
cluster_usages.py

For each initial, middle, and final cluster found in data/stened.json,
produce a sample of ~12 example outlines. Multi-stroke outlines are included:
a cluster is matched if it appears in any stroke of the outline.

Output: build/cluster_usages.json
"""

import json
import os
import random
from collections import defaultdict

STENED_PATH = "data/stened.json"
OUTPUT_PATH = "build/cluster_usages.json"
SAMPLE_SIZE = 20

LEFT_KEYS  = set("STKPWHR")
MIDDLE_KEYS = set("AO*EU")
RIGHT_KEYS  = set("FRPBLGTSDZ")


def parse_stroke(s):
    """Return (initial_str, middle_str, final_str) for a single stroke.

    Returns None if the stroke contains digits, '#', or unrecognised characters.
    Initials are the left-hand cluster (e.g. 'SK'), middles are the vowel keys
    (e.g. 'AO'), finals are the right-hand cluster (e.g. 'FP').
    """
    if any(c.isdigit() or c == '#' for c in s):
        return None

    left_side = True
    initials = []
    middles = []
    finals = []

    for ch in s:
        if ch == '-':
            left_side = False
            continue
        if ch in MIDDLE_KEYS:
            left_side = False
            middles.append(ch)
        elif left_side:
            if ch not in LEFT_KEYS:
                return None
            initials.append(ch)
        else:
            if ch not in RIGHT_KEYS:
                return None
            finals.append(ch)

    return ''.join(initials), ''.join(middles), ''.join(finals)


def main():
    with open(STENED_PATH, encoding="utf-8") as f:
        stened = json.load(f)

    # Maps from cluster string -> {outline: word}
    initial_outlines = defaultdict(dict)
    middle_outlines  = defaultdict(dict)
    final_outlines   = defaultdict(dict)

    for outline, word in stened.items():
        strokes = outline.split('/')

        for stroke in strokes:
            parsed = parse_stroke(stroke)
            if parsed is None:
                continue

            initial, middle, final = parsed

            if initial:
                initial_outlines[initial][outline] = word
            if middle:
                middle_outlines[middle][outline] = word
            if final:
                # Prefix finals with '-' to distinguish them from initials
                final_outlines['-' + final][outline] = word

    # Build output: for each cluster, pick up to SAMPLE_SIZE random examples.
    # Clusters are ordered by how many words contain them (most frequent first).
    result = {}

    for cluster_group in (initial_outlines, middle_outlines, final_outlines):
        for cluster, outlines in sorted(cluster_group.items(), key=lambda x: -len(x[1])):
            items = list(outlines.items())
            random.shuffle(items)
            result[cluster] = dict(items[:SAMPLE_SIZE])

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    with open(OUTPUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(result, f, indent=4, ensure_ascii=False)

    print(f"Written {len(result)} clusters to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
