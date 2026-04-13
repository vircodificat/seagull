"""Examples of the seagull Python library."""

from seagull import Key, Stroke, Outline

# ---------------------------------------------------------------------------
# Constructing strokes and outlines
# ---------------------------------------------------------------------------

# A stroke is a single simultaneous key press.
kat = Stroke('KAT')
assert repr(kat) == "Stroke('KAT')"
assert str(kat)  == 'KAT'

# An outline is one or more strokes separated by /.
outline = Outline('KAT/ER')
assert repr(outline) == "Outline('KAT/ER')"
assert str(outline)  == 'KAT/ER'

# A single-stroke outline is also valid.
assert repr(Outline('TEFT')) == "Outline('TEFT')"

# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------

# Stroke rejects any letter that isn't a valid steno key.
# (Valid steno letters: S T K P W H R A O * E U F R P B L G T S D Z)
for s, msg in [('CAT',    "Invalid stroke 'CAT'"),
               ('FOO',    "Invalid stroke 'FOO'"),   # F is right-hand; can't precede O
               ('KAT/ER', "Invalid stroke 'KAT/ER'")]:  # ← slash makes it an outline
    try:
        Stroke(s)
        assert False, f"expected ValueError for {s!r}"
    except ValueError as e:
        assert str(e) == msg, str(e)

# Outline accepts multi-stroke strings but rejects invalid strokes within them.
try:
    Outline('KAT/FOO')
    assert False, "expected ValueError"
except ValueError:
    pass

# Rejects initial F (should be TP)
try:
    Stroke('FAT')
    assert False, "expected ValueError"
except ValueError:
    pass

# Rejects keys in wrong order: PT should be TP
try:
    Stroke('PTAT')
    assert False, "expected ValueError"
except ValueError:
    pass

# ---------------------------------------------------------------------------
# Iterating an outline gives strokes
# ---------------------------------------------------------------------------

assert [str(s) for s in Outline('PHERPB/SHUS')] == ['PHERPB', 'SHUS']

# ---------------------------------------------------------------------------
# Iterating a stroke gives keys
# ---------------------------------------------------------------------------

assert list(Stroke('STPH')) == [Key.LeftS, Key.LeftT, Key.LeftP, Key.LeftH]

# ---------------------------------------------------------------------------
# str() on a Key gives the bare letter
# ---------------------------------------------------------------------------

assert str(Key.LeftK)      == 'K'
assert str(Key.RightT)     == 'T'
assert str(Key.MiddleStar) == '*'

# Collect the letters of a stroke as a list of strings.
assert [str(k) for k in Stroke('STPH')] == ['S', 'T', 'P', 'H']

# ---------------------------------------------------------------------------
# Decomposing a stroke into initials, middles, finals
# ---------------------------------------------------------------------------

s = Stroke('STRAP')
assert str(s.initials()) == 'STR'
assert str(s.middles())  == 'A'
assert str(s.finals())   == '-P'

# A stroke with only right-hand keys.
assert str(Stroke('-FPL').initials()) == ''
assert str(Stroke('-FPL').finals())   == '-FPL'

# A stroke with only vowels.
assert str(Stroke('AOE').initials()) == ''
assert str(Stroke('AOE').middles())  == 'AOE'

# ---------------------------------------------------------------------------
# keys() and strokes() return plain lists
# ---------------------------------------------------------------------------

assert Stroke('KAT').keys() == [Key.LeftK, Key.MiddleA, Key.RightT]

assert Outline('KAT/ER').strokes() == [Stroke('KAT'), Stroke('ER')]

# ---------------------------------------------------------------------------
# Strokes and Outlines are hashable — usable as dict keys / set members
# ---------------------------------------------------------------------------

lookup = {
    Outline('KAT'):    'cat',
    Outline('HO*US'):  'house',
    Outline('TEFT'):   'test',
}
assert lookup[Outline('KAT')]  == 'cat'
assert lookup[Outline('TEFT')] == 'test'

seen = {Stroke('KAT'), Stroke('KAT'), Stroke('ER')}
assert len(seen) == 2   # duplicates collapsed

# ---------------------------------------------------------------------------
# Equality
# ---------------------------------------------------------------------------

assert Stroke('KAT') == Stroke('KAT')
assert Stroke('KAT') != Stroke('TEFT')
assert Outline('KAT/ER') == Outline('KAT/ER')

# ---------------------------------------------------------------------------
# Building an outline programmatically from its strokes
# ---------------------------------------------------------------------------

words = ['KAT', 'ER']
strokes = [Stroke(w) for w in words]
rebuilt = Outline('/'.join(str(s) for s in strokes))
assert rebuilt == Outline('KAT/ER')
