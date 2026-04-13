# Seagull Steno Theory — Comprehensive Plan

## 1. What This Document Is

A working plan for building a personal steno theory and dictionary, using `data/stened.json`
as the phonetic backbone, `data/main.json` as a reference for coverage and conflict checking,
and `data/seagull_base.json` as the hand-crafted seed of deliberate design decisions.
The goal is a theory that is **regular enough to learn by rule**, **brief enough to be fast**,
**conflict-free enough to be reliable**, and **open enough to grow** via tooling and ML.

---

## 2. Understanding the Starting Material

### 2.1 stened.json (70,144 entries)
A court-reporting dictionary (Stened theory). Key characteristics:
- **82% multi-stroke** (57,834 / 70,144). Heavy use of 2- and 3-stroke spellings.
- **Phrase strokes** are first-class: `STPH-T` → "isn't the", `STPARS` → "as far as".
  This reflects court-reporting priorities (speed on fixed legal phrases) that may not
  suit general writing.
- **Star for disambiguation**: ~3,000 pairs where adding `*` changes meaning—most commonly
  capitalisation (`STPHART` → "senator" / `STPHA*RT` → "Senator"), tense
  (`STHAEPBDZ` → "that she understand" / `STHA*EPBDZ` → "that she understood"),
  and negation.
- **Good phonetic regularity** for single words; the consonant-cluster mappings
  (STP=sf/sph, STPHR=sn, etc.) are internally consistent.

### 2.2 main.json (147,424 entries)
Likely derived from the Plover/Lapwing ecosystem. Key characteristics:
- Roughly twice the coverage. Worth mining for words stened omits entirely.
- **2,251 conflicting assignments** for the same outline. Where stened and main disagree,
  seagull must make an explicit choice (see §5).
- Heavy use of number-key chords for time, currency, and legal abbreviations—largely
  irrelevant to a general-purpose theory and safe to ignore.
- A useful source of **suffix and prefix strokes** (e.g. `APBS` → `{^ance}`).

### 2.3 seagull_base.json (hand-crafted seed)
Already encodes several deliberate theory decisions worth preserving and extending:
- **Pronoun + `*` = contraction**: `EU`=I → `*EUPL`=I'm, `*EUF`=I've, `*EUL`=I'll, `*EUD`=I'd.
  Same pattern for you/he/she/it/we/they. This is a strong, learnable rule.
- **`*PB` as the n't morpheme**: `K*PB`=can't, `R*PB`=aren't. Composable with any modal.
- **Right-hand-only suffix strokes**: `-R`={^er}, `-S`={^s}, `-G`={^ing}, `-D`={^ed},
  `-RS`={^ers}, `-RG`={^ering}, `HREU`={^ly}.
- **Reflexives built from pronouns**: `PHEU/SEF`=myself, `UR/SEF`=yourself.
- **`-F` = "of"** (not stened's `OF`). A deliberate brief worth keeping.

---

## 3. Core Theory Decisions

### 3.1 Phonetic Principle
The theory is **phonetic-first, spelling-second**. Outlines represent how a word sounds,
not how it is spelled. Spelling-based alternates may exist as secondary strokes but are
never the canonical form. This aligns with stened's approach and makes the theory
learnable from phonetics alone.

### 3.2 The Steno Keyboard Layout
```
Left hand    Vowels    Right hand
S T K P W H R   A O * E U   F R P B L G T S D Z
```
Left-hand consonant clusters are formed by pressing multiple keys simultaneously.
The canonical left-hand mappings to follow from stened:

| Stroke  | Phoneme(s) |   | Stroke  | Phoneme(s) |
|---------|-----------|---|---------|-----------|
| S       | s         |   | STPH    | sn         |
| T       | t         |   | STPHR   | sn- (alt)  |
| K       | k / c(k)  |   | KP      | x / ex-    |
| P       | p         |   | KW      | qu         |
| W       | w         |   | SKWR    | j          |
| H       | h         |   | TPH     | n          |
| R       | r         |   | TK      | d          |
| TP      | f         |   | PW      | b          |
| TH      | th        |   | TKPW    | g          |
| PH      | m         |   | HR      | l          |
| KR      | cr / c    |   | SR      | v          |
| WR      | wr / r    |   | SH      | sh         |

Right-hand consonant clusters similarly, forming the coda of each syllable.

### 3.3 Vowel System
The vowel keys `A O * E U` and their combinations:

| Stroke | Sound   | Example           |
|--------|---------|-------------------|
| A      | short a | cat               |
| O      | short o | cot               |
| E      | short e | bet               |
| U      | short u | but               |
| EU     | short i | bit               |
| AO     | long oo | boot              |
| AE     | long a  | bait (alternate)  |
| AOE    | long e  | beet              |
| OE     | long o  | boat              |
| AOU    | long u  | cute              |
| OEU    | oi/oy   | boy               |
| OU     | ow      | bout              |
| AU     | aw      | caught            |

The `*` key is **not a vowel modifier** in the core phonetic system. Its roles are:
1. Disambiguation (homophone resolution)
2. Capitalisation of proper nouns
3. Contraction formation (see §3.5)

### 3.4 Suffix System (Right-hand strokes)
Suffixes attach to word strokes using right-hand-only chords or short follow-on strokes.
The seagull_base system is extended:

| Stroke  | Suffix    | Notes                        |
|---------|-----------|------------------------------|
| `-S`    | {^s}      | Plural, 3rd-person singular  |
| `-D`    | {^ed}     | Past tense                   |
| `-G`    | {^ing}    | Progressive                  |
| `-R`    | {^er}     | Comparative, agentive        |
| `-RS`   | {^ers}    | Agentive plural              |
| `-RG`   | {^ering}  | Progressive of -er verbs     |
| `HREU`  | {^ly}     | Adverb                       |
| `-BL`   | {^able}   | (from stened: A-BL = able)   |
| `-PB`   | {^en}     | Past participle (stened)     |
| `-PBS`  | {^ens}    | (stened)                     |

### 3.5 Contraction System
The seagull_base pattern is elevated to a first-class rule:

> **Rule**: Any subject-pronoun stroke + `*` + right-hand contraction suffix = contraction.

Contraction suffixes used:
- `PL` = 'm  (`*EUPL` = I'm)
- `F` = 've  (`*EUF` = I've)
- `L` = 'll  (`*EUL` = I'll)
- `D` = 'd   (`*EUD` = I'd)
- `S` = 's   (`H*ES` = he's)

For negative contractions, the rule is:
> **Rule**: any modal/auxiliary stroke + `*PB` = contracted negation.

`K*PB`=can't, `TKO*PB`=don't, `W*PB`=won't, `W*PBD`=wouldn't, `K*PBD`=couldn't,
`SH*PBD`=shouldn't, `S*PB`=isn't, `W*PBS`=wasn't, etc.

This system is entirely learnable from two rules, not memorised entry-by-entry.

### 3.6 Star Key Usage — Priority Order
The `*` key has multiple overloaded roles. Seagull uses this strict priority:
1. **Undo** (hardware/engine level — reserved, never in dictionary)
2. **Contraction** (pronoun + `*` + suffix, see §3.5)
3. **Negation** (auxiliary + `*PB`)
4. **Capitalisation** of proper noun (`STPHAT`=senate / `STPHA*T`=Senate)
5. **Homophone disambiguation** (`KAT`=cat / `KA*T` = the rarer homophone)
6. **Tense shift** (e.g. present→past for irregular verbs)

Role 1 is engine-level; roles 2–3 are systematic rules; roles 4–6 require per-word
assignment but follow the same user mental model: star = "something different about this."

---

## 4. Brief Word System

### 4.1 Principles
- The top ~500 words by frequency should be achievable in a single stroke.
- A brief should be **phonetically motivated** where possible, not arbitrary.
- Where a brief conflicts with a phonetic word, the brief wins for the more common word;
  the rarer phonetic word gets a secondary stroke.
- Briefs should **not** occupy star-key slots unnecessarily — leave those for contractions.

### 4.2 Established Briefs (from seagull_base, confirmed good)
```
T=the    A=a     S=is    EU=I    PW=be   K=can   R=are   W=with
-F=of    -T=it   -PB=in  TP=if   TH=this THA=that THE=they
TO=to    HE=he   SHE=she WE=we   U=you   PHE=me   HR=here
```

### 4.3 Conflict Resolution Policy
When stened and main.json disagree (2,251 cases), resolution order:
1. If seagull_base has already made the choice, keep it.
2. Prefer the **more common English word** (verified via word frequency lists).
3. Prefer the **phonetically cleaner** mapping.
4. Give the displaced word its full phonetic stroke (2-stroke if needed).

Key examples already resolved in seagull_base:
- `OF`=off (not stened's "of"), `-F`=of — cleaner brief assignment
- `HR`=here (not stened's "will"); `W-L`=will — more phonetic for will
- `-PB`=in (overrides stened's `{^en}`) — "in" is vastly more common

### 4.4 Words Still Needing Single-Stroke Briefs
Priority: assign a single-stroke brief to every word in the top 300 by frequency.
Currently ungapped: not, all, so, no, know, think, come, look, good, new, want, give,
use, find, tell, work, call, try, ask, seem, feel, leave, keep, begin, show, hear, play.

---

## 5. Conflict Map and Resolution Process

2,251 outline conflicts exist between stened and main. The process:
1. Extend `main.py` to emit a TSV: `outline | stened_word | main_word`.
2. Label each row with word frequency (wordfreq library or corpus counts).
3. Auto-assign the more common word; queue the rarer for a new phonetic stroke.
4. Hand-review any case where both words are in the top 10,000 by frequency.
5. Record every decision in `data/conflict_resolutions.json` as an audit trail.

---

## 6. Multi-Stroke (Phonetic Spelling) Strategy

For words not covered by a brief, the theory falls back to **phoneme-by-phoneme strokes**
using CMUdict pronunciations as the authoritative source.

### 6.1 Syllable Mapping
Each stroke covers one syllable: **onset** (left-hand cluster) + **nucleus** (vowel chord)
+ **coda** (right-hand cluster). When a phoneme sequence violates key order (e.g. a coda
consonant that would need a left-hand key), it forces a new stroke.

### 6.2 Coda Clusters (right-hand)

| Stroke  | Phoneme  |   | Stroke   | Phoneme    |
|---------|---------|---|----------|------------|
| `-F`    | -f, -v  |   | `-PB`    | -n         |
| `-P`    | -p      |   | `-PBG`   | -nk        |
| `-B`    | -b      |   | `-FP`    | -ch        |
| `-L`    | -l      |   | `-RB`    | -sh        |
| `-G`    | -g, -ng |   | `-PBLG`  | -j, -ge    |
| `-T`    | -t      |   | `-BG`    | -k         |
| `-S`    | -s, -z  |   | `-BGS`   | -x, -ks    |
| `-D`    | -d      |   | `-FPL`   | -m         |

### 6.3 The CMUdict Pipeline
`data/cmudict/cmudict` maps words to ARPAbet phoneme sequences. The pipeline:

1. Look up word in CMUdict → get phoneme sequence.
2. Split into syllables (onset / nucleus / coda).
3. Map each phoneme to its steno key(s) via a rule table.
4. Check the generated outline against the existing dictionary for conflicts.
5. If conflict-free, add automatically. If conflict, flag for manual review.

This pipeline can produce the bulk of the English dictionary automatically and forms
the foundation for the ML-assisted generation described in §8.


---

## 7. Learnability and Teachability

A theory is only as good as how quickly someone can internalise and extend it.

### 7.1 Rule Hierarchy
Present the theory in layers so a learner has a working dictionary from day one:

| Layer | What you learn | Coverage |
|-------|---------------|----------|
| 1 | ~150 brief words from seagull_base | ~50% of words in typical text |
| 2 | Suffix strokes (-S, -D, -G, -R, -RS, HREU) | Inflections of layer-1 words |
| 3 | Contraction system (pronoun + * + suffix) | All contractions |
| 4 | Negation system (auxiliary + *PB) | All negative contractions |
| 5 | Phonetic vowel system (12 vowel chords) | Any syllable nucleus |
| 6 | Left-hand onset clusters | Any syllable onset |
| 7 | Right-hand coda clusters | Any syllable coda |
| 8 | Multi-stroke spelling rules | Any word in CMUdict |
| 9 | Disambiguation / * patterns | Homophones, proper nouns |

Someone at layer 3 after a week of practice can write most conversational English.
Layers 5–8 take months, but they are **learnable by rule**, not by rote.

### 7.2 Mnemonic Anchors
- Contraction: the star always means "I'm doing something to this word" — shrinking it.
- Negation: `*PB` looks like "no/n't" — the B and PB cluster sounds like "nuh".
- `-D`=past, `-G`=going (present progressive), `-S`=plural: these mirror common suffixes.
- Brief motivation: `T`=the (sounds like a schwa "thuh"), `S`=is (sibilant start).

### 7.3 Regularity vs. Historical Compatibility
Stened carries court-reporting legacy (Q/A formatting strokes, legal phrase clusters,
`STKPWHR-` prefix for transcript lines). These should be **sequestered** into a
separate `data/commands.json` layer rather than polluting the main phonetic dictionary.
Importing stened wholesale preserves history but obscures regularity. The recommended
approach: import stened's **phonetic word entries only** (filter out entries that contain
phrases with spaces, entries starting with numbers, and entries with formatting commands).

---

## 8. Machine Learning Opportunities

The CMUdict pipeline (§6.3) plus the existing dictionary data creates a rich training
environment. Here are concrete ML applications for both dictionary building and the
steno software engine.

### 8.1 Outline Generation via Sequence-to-Sequence Models
**Goal**: Given a word (or its phoneme sequence), predict the best steno outline.

**Data**: The existing ~70,000 (stened) + ~147,000 (main) word→outline pairs provide
a large supervised dataset. Each entry maps a word/phoneme sequence to a stroke sequence.

**Architecture**: A small seq2seq model (encoder-decoder with attention, or a character-level
transformer) trained on `phoneme_sequence → stroke_sequence`. The input is the CMUdict
ARPAbet sequence; the output is the steno stroke string.

**Why this works**: The mapping is highly structured (not arbitrary), so even a small model
can generalise. The main value is handling words not in CMUdict (proper nouns, neologisms)
and learning implicit theory conventions that are hard to specify as rules.

**Training data prep**:
1. Join stened/main with CMUdict on word → get (phoneme_seq, outline) pairs.
2. Filter to single-word, single-meaning entries.
3. Train 80/10/10 split; evaluate by exact-match outline accuracy and "valid stroke" rate.

### 8.2 Conflict Prediction
**Goal**: Before adding a new entry, predict whether its proposed outline will conflict
with existing entries or with future CMUdict-generated entries.

**Approach**: Train a classifier on outline strings → probability of conflict.
Features: stroke length, key density, presence of *, vowel chord, left/right cluster.
This catches the systematic conflict patterns (e.g. right-hand-only strokes are heavily
contested) before manual entry.

### 8.3 Frequency-Weighted Brief Assignment
**Goal**: Given a pool of candidate outlines for a word, rank them by stroke economy
weighted by word frequency.

**Metric**: `strokes_per_word × inverse_rank_in_frequency_list`. Optimise the whole
dictionary jointly — a brief that saves 0.3 strokes on a top-10 word is worth more
than one that saves 1.5 strokes on a top-10,000 word.

**Algorithm**: Treat brief assignment as a minimum-cost bipartite matching problem.
Words are nodes on one side; candidate outlines on the other. Edge weight = expected
strokes-per-occurrence over a large corpus. Run Hungarian algorithm or a greedy
frequency-ordered assignment.

### 8.4 Adaptive/Personalised Suggestion
**Goal**: As the user types, the engine observes which outlines they use, which they
misstroke, and which alternative outlines they reach for. Over time, suggest reassignments.

**Approach**:
- Log all strokes + corrections (undo events).
- Words with high undo rate → the current outline is hard for this user's hands.
- Offer alternative outlines (from the star-disambiguation space or 2-stroke options).
- A small online model (e.g. logistic regression on stroke-feature vectors) predicts
  which candidate a specific user will prefer.

### 8.5 Next-Word / Phrase Prediction
**Goal**: The engine predicts the next word given context and pre-populates a phrase
brief for the current moment.

**Approach**: A bigram or small n-gram language model (or a distilled LLM) runs in the
background. When `P(next_word | context) > threshold`, the engine lights up a chord
that would output `current_word + next_word` in one stroke — a dynamic brief.

This is the most powerful ML feature for raw speed. Even a trigram model trained on
the user's own writing corpus can dramatically reduce strokes-per-word in repetitive
domains (legal writing, medical notes, personal emails).

### 8.6 Phoneme-to-Steno Rule Extraction
**Goal**: Rather than hand-writing the phoneme→key mapping tables (§6.2), learn them
from the existing dictionary.

**Approach**: Align (phoneme_seq, outline) pairs at the phoneme level using a
monotonic alignment algorithm (similar to CTC or DTW alignment). Extract the most
probable phoneme→key mapping for each ARPAbet phoneme. This produces a data-driven
rule table that can be compared to the hand-written one to find inconsistencies or
missed patterns in the theory.

---

## 9. Software Architecture Notes (Seagull Engine)

The Rust engine in `seagull/src/lib.rs` already has the key data structures. Extensions
that would support the above:

- **`Stroke::initials/middles/finals`** (already implemented): enables phonemic analysis
  of strokes in code — useful for conflict detection and ML feature extraction.
- **Stroke log**: persist every stroke (with timestamp, context word, undo flag) to a
  local SQLite or append-only log file. This is the data source for §8.4 and §8.5.
- **Multi-dictionary layers**: load seagull_base first (highest priority), then the
  generated phonetic dictionary, then stened as fallback. A `Layer` enum on each entry
  makes priority explicit and auditable.
- **Outline suggestion API**: given a word string, return the top-N candidate outlines
  ranked by the model from §8.3. Expose this to a UI for interactive brief assignment.
- **Real-time conflict checker**: when the user manually adds an entry (via a UI or
  config file), the engine immediately reports any outline that would shadow an existing
  entry at a higher-priority layer.
- **CMUdict integration**: the `data/cmudict` directory is already present. A Rust
  module (or Python script in `main.py`) should parse it and expose a
  `word → Vec<Vec<Phoneme>>` lookup, one pronunciation per variant.

---

## 10. Immediate Next Steps

1. **Audit brief gaps** (§4.4): list the top-300 words missing single-stroke briefs,
   assign them, add to `seagull_base.json`. Verify no conflicts with existing entries.

2. **Build the conflict TSV** (§5): extend `main.py` to emit `conflict_resolutions.json`
   and resolve the 2,251 stened/main conflicts systematically.

3. **Implement CMUdict pipeline** (§6.3): write a Python script that takes a word list,
   looks each up in `data/cmudict/cmudict`, and generates candidate outlines using the
   phoneme→key table. Output a `data/generated_phonetic.json`.

4. **Harden the contraction/negation rules** (§3.5–3.6): enumerate all subject pronouns
   and all modal/auxiliary verbs and generate the full set of contraction and negation
   entries programmatically. Add to `seagull_base.json` or a dedicated
   `data/contractions.json`.

5. **Collect a personal corpus**: write 10,000+ words of your own text through the
   engine (even if slowly), logging every stroke. This seeds the personalisation model
   in §8.4 and reveals which theory decisions cause the most friction in practice.

6. **Train a pilot seq2seq model** (§8.1): use the stened entries that have CMUdict
   coverage (~40,000 entries) as training data. Evaluate on held-out words. A working
   model validates that the theory is regular enough to be learned by a neural net —
   which is a good proxy for it being learnable by a human too.

