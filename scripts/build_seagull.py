"""Build seagull.json by starting from BASE_PATH and merging dictionaries
listed in EXTRA_DICTS in order.  Outlines already present are never overwritten.

Output: build/seagull.json
"""

import json
import os

BASE_PATH   = "data/seagull_base.json"
OUTPUT_PATH = "build/seagull.json"

# Dictionaries to merge in order (highest priority first).
# Outlines already present in an earlier entry are skipped.
EXTRA_DICTS = [
    "data/commands.json",
    "data/punctuation.json",
    "build/obvious_outlines.json",
    "build/reasonable_outlines.json",
]


def main():
    result: dict[str, str] = json.load(open(BASE_PATH, encoding="utf-8"))
    print(f"{BASE_PATH}: {len(result)} entries")

    for path in EXTRA_DICTS:
        source: dict[str, str] = json.load(open(path, encoding="utf-8"))
        added = 0
        for outline, word in source.items():
            # ----------------------------------------------------------------
            # Outlines starting with "A/" are rewritten to "A*/".
            # ----------------------------------------------------------------
            if outline.startswith("A/"):
                outline = "A*/" + outline[2:]

            if outline not in result:
                result[outline] = word
                added += 1
        print(f"{path}: added {added} of {len(source)}")

    os.makedirs("build", exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"Total: {len(result)} entries written to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
