"""
Convert a CMU pronunciation list to a steno outline (one syllable only).

RULES
=====
Steno layout (left→right): S T K P W H R | A O * E U | -F -R -P -B -L -G -T -S -D -Z

INITIAL CONSONANTS  (pre-vowel CMU phoneme → left-side steno keys)
  B   → PW     CH  → KH     D   → TK     DH  → TKH
  F   → TP     G   → TKPW   HH  → H      JH  → SKWR
  K   → K      L   → HR     M   → PH     N   → TPH
  P   → P      R   → R      S   → S      SH  → SH
  T   → T      TH  → TH     V   → SR     W   → W
  Y   → KWR
Clusters: OR the key sets of each phoneme in order. If any two phonemes share
a key the cluster is unrepresentable → None.

VOWELS  (stress digit stripped first; CMU base phoneme → steno middle keys)
  AE  → A         AH  → U         AW  → OU        AY  → AOEU
  EY  → AEU       OW  → OE        OY  → OEU        UW  → AO   (covers "oo"-spelled words: boot, boom, boon; "ew"/"ue" words
              such as brew, clue use AOU in practice — those are not predicted
              correctly because the distinction is spelling-dependent)
  EH  → E   when the first coda phoneme is NOT R; else → None
              (bare EH1 R → PWAEUR vs bear EH1 R → PWAER are homophones
               disambiguated by spelling; unknowable from phonemes alone)
  IH  → EU  when the first coda phoneme is NOT R; else → None
              (dear IH1 R → TKAER vs deer IH1 R → TKAOER, same reason)
  AA  → A   when the first coda phoneme is R (the "ar" vowel: bar, arm, cart)
       → O   otherwise (the "short-o" vowel: hot, cop, box)
  AO  → None  (three steno representations — O, AU, AO — depending on the word;
               no rule derivable from phonemes alone)
  ER  → None  (three representations — E, EU, U — reflect English spelling
               of "er"/"ir"/"ur"; unknowable from phonemes alone)
  IY  → None  (sea → SAE but see → SAOE; AE vs AOE distinction requires
               knowing which homophone is intended)
  UH  → None  (bull → U but book → AO; no phoneme-based rule)

FINAL CONSONANTS  (post-vowel CMU phoneme → right-side steno keys)
  SPECIAL: a final T immediately followed by CH is dropped; the CH alone
  provides -FP (e.g. blotch B L AA1 T CH → PWHROFP, not PWHROFPT).
  This matches the standard steno convention that -tch is written as -FP.
  B   → -B      CH  → -FP     D   → -D      F   → -F
  G   → -G      JH  → -PBLG   K   → -BG     L   → -L
  M   → -PL     N   → -PB     NG  → -PBG    P   → -P
  R   → -R      S   → -S      SH  → -RB     T   → -T
  Z   → -Z
Clusters: OR the key sets. Shared key → None.

STROKE RENDERING
  Left keys printed in order: S T K P W H R
  Middle keys printed in order: A O * E U
  Right keys printed in order: F R P B L G T S D Z
  When no middle keys are pressed, a '-' separates left from right.

LIMITATIONS
  Returns None for AO, ER, IY, UH vowels and for EH/IH before coda R.
  Returns None for any unknown or unrepresentable phoneme.
  Does not handle multi-syllable input.
  UW → AO is correct for "oo"-spelled words but wrong for "ew"/"ue" words.
  Homophones that steno disambiguates by outline (e.g. bread/bred, ale/ail,
  cell/sell) cannot be resolved from phonemes alone; one of the pair will
  receive the wrong outline.
"""

import sys

# ── steno key orderings ────────────────────────────────────────────────────────
LEFT_ORDER  = list("STKPWHR")
MID_ORDER   = list("AO*EU")
RIGHT_ORDER = list("FRPBLGTSDZ")

# ── phoneme → steno key mappings ──────────────────────────────────────────────
INIT_KEYS: dict[str, frozenset[str]] = {
    "B":  frozenset("PW"),  "CH": frozenset("KH"),
    "D":  frozenset("TK"),  "DH": frozenset("TKH"),
    "F":  frozenset("TP"),  "G":  frozenset("TKPW"),
    "HH": frozenset("H"),   "JH": frozenset("SKWR"),
    "K":  frozenset("K"),   "L":  frozenset("HR"),
    "M":  frozenset("PH"),  "N":  frozenset("TPH"),
    "P":  frozenset("P"),   "R":  frozenset("R"),
    "S":  frozenset("S"),   "SH": frozenset("SH"),
    "T":  frozenset("T"),   "TH": frozenset("TH"),
    "V":  frozenset("SR"),  "W":  frozenset("W"),
    "Y":  frozenset("KWR"),
}

FINAL_KEYS: dict[str, frozenset[str]] = {
    "B":  frozenset("B"),   "CH": frozenset("FP"),
    "D":  frozenset("D"),   "F":  frozenset("F"),
    "G":  frozenset("G"),   "JH": frozenset("PBLG"),
    "K":  frozenset("BG"),  "L":  frozenset("L"),
    "M":  frozenset("PL"),  "N":  frozenset("PB"),
    "NG": frozenset("PBG"), "P":  frozenset("P"),
    "R":  frozenset("R"),   "S":  frozenset("S"),
    "SH": frozenset("RB"),  "T":  frozenset("T"),
    "Z":  frozenset("Z"),
}

VOWEL_SIMPLE: dict[str, str] = {
    "AE": "A",  "AH": "U",  "AW": "OU",  "AY": "AOEU",
    "EY": "AEU","OW": "OE", "OY": "OEU", "UW": "AO",
}


def pronunciation_to_outline(phonemes: list[str]) -> str | None:
    """Return the steno outline for a CMU phoneme list, or None if not obvious.

    Args:
        phonemes: list of CMU phonemes, e.g. ['K', 'AE1', 'T'] for "cat".

    Returns:
        A steno outline string (e.g. 'KAT'), or None when no obvious outline
        exists (ambiguous vowel, unknown phoneme, or key conflict in cluster).
    """
    # Locate the single vowel (phoneme whose last character is a stress digit).
    vowel_idx = None
    for i, ph in enumerate(phonemes):
        if ph and ph[-1].isdigit():
            if vowel_idx is not None:
                return None  # more than one vowel → multi-syllable
            vowel_idx = i
    if vowel_idx is None:
        return None

    init_phs  = phonemes[:vowel_idx]
    vowel_ph  = phonemes[vowel_idx]
    final_phs = phonemes[vowel_idx + 1:]
    vowel_base = vowel_ph.rstrip("012")

    # ── vowel → middle keys ───────────────────────────────────────────────────
    if vowel_base in VOWEL_SIMPLE:
        middle = VOWEL_SIMPLE[vowel_base]
    elif vowel_base == "AA":
        middle = "A" if (final_phs and final_phs[0] == "R") else "O"
    elif vowel_base == "EH":
        if final_phs and final_phs[0] == "R":
            return None  # bare/bear ambiguity
        middle = "E"
    elif vowel_base == "IH":
        if final_phs and final_phs[0] == "R":
            return None  # dear/deer ambiguity
        middle = "EU"
    else:
        return None  # AO, ER, IY, UH — not obvious

    # ── initial consonants → left keys ───────────────────────────────────────
    left_keys: set[str] = set()
    for ph in init_phs:
        if ph not in INIT_KEYS:
            return None
        new = INIT_KEYS[ph]
        if left_keys & new:
            return None  # key conflict in cluster
        left_keys |= new

    # ── final consonants → right keys ────────────────────────────────────────
    # Preprocess: a bare T immediately before CH is absorbed into CH (-FP).
    # CMU transcribes -tch as T CH (e.g. blotch = ...T CH), but steno only
    # uses -FP for the whole cluster.
    processed_finals: list[str] = []
    i = 0
    while i < len(final_phs):
        # A bare T immediately before CH is dropped: steno -FP covers the whole
        # -tch cluster (blotch T CH → -FP, not -FPT).
        if final_phs[i] == "T" and i + 1 < len(final_phs) and final_phs[i + 1] == "CH":
            pass  # skip T; the following CH iteration will add -FP
        else:
            processed_finals.append(final_phs[i])
        i += 1

    right_keys: set[str] = set()
    for ph in processed_finals:
        if ph not in FINAL_KEYS:
            return None
        new = FINAL_KEYS[ph]
        if right_keys & new:
            return None  # key conflict in cluster
        right_keys |= new

    # ── render stroke ─────────────────────────────────────────────────────────
    mid_keys  = set(middle)
    left_str  = "".join(k for k in LEFT_ORDER  if k in left_keys)
    mid_str   = "".join(k for k in MID_ORDER   if k in mid_keys)
    right_str = "".join(k for k in RIGHT_ORDER if k in right_keys)

    if mid_str:
        return left_str + mid_str + right_str
    elif right_str:
        return left_str + "-" + right_str
    else:
        return left_str or None


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: pron_to_outline.py PHONEME [PHONEME ...]")
        print("Example: pron_to_outline.py K AE1 T")
        sys.exit(1)
    phonemes = sys.argv[1:]
    result = pronunciation_to_outline(phonemes)
    if result is None:
        print("(no obvious outline)")
    else:
        print(result)


if __name__ == "__main__":
    main()
