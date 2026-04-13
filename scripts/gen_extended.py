"""Generate NOTES/extended.txt — exhaustive lists of every valid
initials, middles, and finals key combination.

  Initials : all 2^7 - 1 = 127  non-empty subsets of { S T K P W H R }
  Middles  : all 2^5 - 1 = 31   non-empty subsets of { A O * E U }
  Finals   : all 2^10 - 1 = 1023 non-empty subsets of { F R P B L G T S D Z }

Subsets are listed in key-order (lowest-bit key first), enumerated in
ascending bitmask order so single keys come first, then pairs, etc.
Finals are prefixed with '-' to distinguish them from initials.
"""

import os

OUTPUT_PATH = os.path.join("build", "extended.txt")

LEFT_KEYS   = list("STKPWHR")
MIDDLE_KEYS = ["A", "O", "*", "E", "U"]
RIGHT_KEYS  = list("FRPBLGTSDZ")


def subsets(keys: list[str]) -> list[str]:
    """Return all non-empty subsets of *keys* in ascending bitmask order."""
    n = len(keys)
    result = []
    for bits in range(1, 1 << n):
        combo = "".join(keys[i] for i in range(n) if bits & (1 << i))
        result.append(combo)
    return result


def main() -> None:
    initials = subsets(LEFT_KEYS)
    middles  = subsets(MIDDLE_KEYS)
    finals   = ["-" + s for s in subsets(RIGHT_KEYS)]

    os.makedirs("build", exist_ok=True)
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        f.write(f"INITIALS  ({len(initials)})\n\n")
        for s in initials:
            f.write(f"    {s}\n")

        f.write(f"\nMIDDLES  ({len(middles)})\n\n")
        for s in middles:
            f.write(f"    {s}\n")

        f.write(f"\nFINALS  ({len(finals)})\n\n")
        for s in finals:
            f.write(f"    {s}\n")

    print(f"Written {len(initials)} initials, {len(middles)} middles, "
          f"{len(finals)} finals to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
