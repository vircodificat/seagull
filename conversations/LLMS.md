# Training a Small LLM on Steno Keystrokes

## What Is Steno Input?

Stenography (steno) is a chorded input system where multiple keys are pressed simultaneously to produce
syllables, words, or phrases in a single "stroke." A stroke is a set of simultaneously pressed keys from
a fixed layout (e.g., the Steno order: `#STKPWHRAO*EUFRPBLGTSDZ`). The output of a stroke is not a
character sequence but a dictionary lookup — a stroke maps to a word or phrase.

This is fundamentally different from sequential character-by-character typing, which has major
implications for LLM training.

---

## 1. Data Representation

### Strokes as Tokens
The most natural unit is the **stroke**, not the character. A stroke like `TEFT` maps to "test." You
have two main representation choices:

- **Raw stroke strings** — treat each unique stroke as an atomic token (e.g., `"TEFT"`, `"HRAO*EUF"`).
  Vocabulary size is bounded by your dictionary (typically 100k–200k entries for large Plover dicts).
- **Broken-down key sets** — encode each stroke as a fixed-length binary vector over the ~23 steno keys.
  This is more structured but loses the "word-level" granularity of strokes.

### Multi-Stroke Words
Many words require multiple strokes (e.g., "algorithm" = `AL/TKPWOR/-PLT`). Your model must handle
**sequences of strokes**, not just single strokes, making this a sequence-to-sequence or
next-stroke-prediction problem.

### Fingerspelling and Briefs
Steno has "briefs" (highly abbreviated, arbitrary strokes for common words) and fingerspelling (one
stroke per letter). Both should be represented in training data — they follow very different statistical
patterns.

---

## 2. Tokenization Strategy

Standard BPE/WordPiece tokenizers are designed for characters and subwords — they are a poor fit for
steno. Consider:

- **Stroke-level vocabulary**: each unique stroke is one token. Works well if your dictionary is fixed.
- **Key-level encoding**: decompose each stroke into its 23 binary key positions. Encodes structure
  explicitly, but sequences become much longer.
- **Hybrid**: a stroke embedding layer that encodes the key bitmap, followed by a positional sequence
  model over strokes. This is probably the best approach for a small model.

Avoid treating steno output text as your primary token stream — the model should reason over *strokes*,
not over the English words they produce.

---

## 3. Dataset Considerations

### Sources
- **Steno dictionaries** (Plover's `main.dict`, community dicts): stroke→translation pairs. Good for
  vocabulary, but not sequential context.
- **Steno transcripts / recordings**: real-world steno output logs with stroke-level timestamps. Rare
  but high value.
- **Synthetic data**: generate plausible stroke sequences from English corpora using a steno engine
  (e.g., Plover in "translation mode"). This is likely your main data source.

### Sequence Length
A steno transcript of normal speech will average ~1 stroke per word. At 150 WPM, that's ~150 strokes
per minute. Context windows of 512–2048 strokes cover several minutes of dictation — plan accordingly.

### Class Imbalance
A small number of brief strokes (e.g., `-T` → "the", `EUPB` → "in") will dominate the distribution.
Apply frequency-aware sampling or loss weighting to prevent the model from collapsing to the most
common strokes.

---

## 4. Task Formulation

Define clearly what you want the model to *do*:

| Task | Input | Output |
|---|---|---|
| Next-stroke prediction | stroke history | next stroke |
| Stroke-to-text | stroke sequence | English text |
| Text-to-stroke | English text | stroke sequence |
| Error correction | malformed stroke sequence | corrected sequence |
| Autocomplete / suggestion | partial stroke sequence | top-k completions |

**Next-stroke prediction** is the most natural LM objective and transfers well to autocomplete and
real-time writing assistance.

---

## 5. Model Architecture

For a *small* model (< 100M parameters):

- **Transformer decoder** (GPT-style) over a stroke token vocabulary is the simplest starting point.
- **Embedding layer**: if using key-bitmap encoding, a small linear projection from a 23-dim binary
  vector into a hidden dimension works well and is parameter-efficient.
- **Context length**: 256–1024 strokes is sufficient for most writing assistance tasks.
- **Positional encoding**: standard learned or RoPE embeddings work fine.

Avoid large hidden dims — steno vocabulary is constrained and the input entropy is lower than natural
language, so a model with `d_model=256–512` and 4–6 layers is likely sufficient.

---

## 6. Evaluation Metrics

- **Perplexity** on held-out stroke sequences — the standard LM metric.
- **Top-1 / Top-5 stroke accuracy** for next-stroke prediction.
- **Word Error Rate (WER)** after translating predicted strokes back to English via the steno dictionary.
- **Brief coverage**: does the model learn to use efficient briefs or does it over-rely on
  multi-stroke spellings?
- **Latency**: for real-time use, measure inference time per stroke prediction.

---

## 7. Steno-Specific Linguistic Considerations

- **Phonetic, not orthographic**: steno is phoneme-based. The same English word can have multiple valid
  stroke representations (e.g., homophones map to the same stroke). Your model must handle this
  one-to-many mapping.
- **Writer-dependent dictionaries**: different stenographers use different personal dictionaries and
  brief sets. A model trained on one writer's strokes may not generalize well. Capture writer identity
  if possible (e.g., a conditioning token or adapter).
- **Real-time constraints**: steno is used live. Inference must happen in < 20ms per stroke to be
  useful in a writing assistance context.
- **Chord ordering**: steno keys within a stroke have a canonical left-to-right order. Enforce or
  canonicalize this in preprocessing — `ST` and `TS` are the same chord and should not be two tokens.

---

## 8. Practical Tips

- Start with a **stroke-prediction** task on synthetic data generated from a large English corpus
  (e.g., Wikipedia) translated through Plover.
- Use **curriculum learning**: train first on the most common 1k strokes, then expand to the full
  vocabulary.
- Consider **contrastive or retrieval augmentation** to help the model distinguish similar strokes that
  map to very different words.
- If the goal is writing assistance (autocomplete), a **retrieval-augmented** approach that combines
  a small neural model with dictionary lookups often outperforms a pure generative approach at this
  scale.
- Log and monitor **brief usage rate** during training as a proxy for whether the model is learning
  efficient steno rather than naive letter-by-letter spelling.

---

## 9. How Much Data Do You Need?

### The Chinchilla Baseline
The Chinchilla scaling law recommends **~20 tokens per parameter** for compute-optimal training.
For a 100M-parameter model that means **~2 billion strokes** for a fully optimal run. In practice,
for a constrained, low-entropy domain like steno (fixed vocabulary, strong phonetic patterns), you can
get *reasonable* results with significantly less:

| Data scale | Strokes | Expected outcome |
|---|---|---|
| Minimum viable | 50–100M | Model learns common strokes and basic bigram patterns |
| Good | 500M–1B | Solid next-stroke prediction, decent brief coverage |
| Chinchilla-optimal | ~2B | Best generalization, strong rare-stroke performance |

### Where Does This Data Come From?
The good news: **you can synthesize essentially unlimited data** by passing a large English text corpus
through a steno engine (e.g., Plover's translation pipeline). English Wikipedia alone is ~4 billion
words → ~4 billion strokes. Generating this takes a few CPU-hours, not months.

Real steno transcripts (court records, CART captioning logs) are rare — realistically you might find
10–100M strokes of real-world data publicly. Use real data for fine-tuning and held-out evaluation;
use synthetic data for the bulk of pre-training.

### Rule of Thumb
> For a first experiment: generate **500M strokes** from Wikipedia + a diverse news corpus.
> That is enough to get a working model and a meaningful loss curve. Scale to 2B for a production run.

---

## 10. Estimated Training Cost on a Single GPU at $2/hr

### The Math

**FLOPs for training** a transformer follow the standard approximation:

```
Total FLOPs ≈ 6 × N × D
```

where `N` = parameters and `D` = training tokens (strokes).

| Token count | FLOPs | Notes |
|---|---|---|
| 500M | 6 × 10^8 × 5×10^8 = **3 × 10^17** | First experiment |
| 1B | **6 × 10^17** | Good quality |
| 2B (Chinchilla) | **1.2 × 10^18** | Optimal |

**GPU throughput at $2/hr** — at this price point you're typically on a V100 (16GB) or A10G (24GB):

| GPU | Peak FP16 | Realistic utilization | Effective TFLOPS |
|---|---|---|---|
| V100 16GB | 112 TFLOPS | ~35% | ~40 TFLOPS |
| A10G 24GB | 125 TFLOPS | ~38% | ~47 TFLOPS |

Using **40 effective TFLOPS** (conservative V100 estimate):

```
Wall-clock time = FLOPs / (effective TFLOPS)
               = FLOPs / (4 × 10^13 FLOPS/s)
```

| Run | FLOPs | Wall-clock time | Cost @ $2/hr |
|---|---|---|---|
| 500M tokens | 3 × 10^17 | ~2.1 hrs | **~$4** |
| 1B tokens | 6 × 10^17 | ~4.2 hrs | **~$8** |
| 2B tokens (Chinchilla) | 1.2 × 10^18 | ~8.3 hrs | **~$17** |

### Reality Check: Multiply by Experimentation

A single clean training run is rarely how things go. Factor in:

| Activity | Multiplier |
|---|---|
| 1 clean production run | 1× ($4–$17) |
| + hyperparameter tuning (3–5 runs) | 3–5× ($12–$85) |
| + ablations, architecture search | 5–10× ($20–$170) |
| Full research-grade campaign | 10–20× ($40–$340) |

### Bottom Line

> A first working model on 500M synthetic strokes costs roughly **$4–$8** in raw GPU time.
> A solid, well-tuned 100M-parameter model including experimentation will run you **$50–$200**
> total — well within reach of a personal project budget.

The dominant cost driver is *not* compute — it's the iteration time to figure out the right
architecture, tokenization strategy, and data mix. Budget more time than money.

---

## 11. How Well Would a Small LLM Do at Stroke Mistake Correction?

**Honest answer: moderate, with a hard ceiling.**

Steno errors are almost always *mechanical*, not semantic. The three dominant error types are:

| Error type | Example | LLM needed? |
|---|---|---|
| Missed key | `TEGT` instead of `TEGT` + left thumb | No — edit distance fixes this |
| Extra key pressed | `STEFT` instead of `TEFT` | No — edit distance fixes this |
| Wrong key (adjacency) | `TEFG` instead of `TEFT` | No — key-distance model fixes this |
| Wrong brief in context | `PHEU` ("my") when "I" was intended | Yes — requires context |
| Phonetic confusion | Two valid strokes, wrong one chosen | Yes — requires context |

The first three categories — which represent the **majority of real-world steno errors** — are purely
mechanical and require no language understanding to fix. A dictionary lookup + key-distance metric
handles them cold, with no training data.

The remaining errors (brief disambiguation, phonetic confusion) are where an LLM actually adds value.
But these are a smaller fraction of total errors, so a small 100M-parameter model would give you:

- **~85–95% correction accuracy on mechanical errors** (but so would a rule-based system)
- **~60–75% accuracy on context-dependent errors** — this is where LLMs genuinely help, but a
  small model with limited context will plateau and struggle with rare briefs or personal dictionaries
- **Net gain over a strong non-LLM baseline: modest** — probably 5–15 percentage points on a mixed
  error benchmark

### The Training Data Problem Makes It Harder

To train a correction model you need **error–correction pairs**: (wrong stroke sequence, right stroke
sequence). Clean steno data is already scarce. Labeled error data is almost nonexistent. You will need
to **synthetically corrupt** clean sequences (random key flips, key additions/removals weighted by
finger adjacency on the steno layout) to generate training pairs. This introduces a mismatch between
synthetic and real error distributions that will hurt real-world performance.

### When a Small LLM Is Worth It

A small LLM for correction makes the most sense if:
- You have access to a specific writer's real steno logs (even 1–5M strokes) for fine-tuning
- Your primary error type is brief disambiguation, not key-miss
- You combine it with a strong rule-based pre-filter that catches mechanical errors first

---

## 12. Non-LLM Approaches to Stroke Mistake Correction

These methods are faster, cheaper, more interpretable, and for the dominant error types, more accurate
than a small LLM.

### 12a. Key-Bitmap Edit Distance (Hamming Distance)

The simplest approach. Each stroke is a 23-bit vector. The Hamming distance between two strokes is the
number of keys that differ. For any input stroke not found in the dictionary:

1. Compute Hamming distance to every dictionary entry.
2. Return the nearest neighbor(s).

Fast with a precomputed index (e.g., BK-tree or ball-tree over binary vectors). Fixes single-key and
two-key errors perfectly. Zero training data required.

**Best for**: pure mechanical correction in isolation (no context).

### 12b. Noisy Channel Model

The classical NLP spell-correction architecture:

```
P(intended | observed) ∝ P(observed | intended) × P(intended)
```

- **Error model** `P(observed | intended)`: probability that stroke `A` was mistyped as stroke `B`,
  based on key adjacency and empirical finger-slip data. Can be estimated from a small annotated set
  or approximated with Hamming distance.
- **Language model** `P(intended)`: an n-gram model over stroke sequences, trained on synthetic steno
  data. KenLM can build a 5-gram model over 500M strokes in minutes.
- **Decoder**: beam search or Viterbi over the stroke sequence finds the globally most likely intended
  sequence, not just per-stroke corrections.

This is how Norvig-style spell correctors and early OCR post-processors work. It handles both
mechanical and mild contextual errors well.

**Best for**: whole-sequence correction where context matters, with modest training data.

### 12c. Weighted Finite State Transducers (WFSTs)

The approach used in production speech recognition systems (Kaldi, ESPnet). Encode the error model and
the n-gram LM as weighted FSTs, then compose them. The resulting transducer maps observed stroke
sequences to corrected ones in a single efficient pass.

- **Pros**: extremely fast at inference, mathematically principled, handles context elegantly.
- **Cons**: non-trivial to build; requires familiarity with OpenFST or similar.

**Best for**: production systems where inference latency and correctness guarantees matter.

### 12d. Hidden Markov Model + Viterbi

Model the steno session as an HMM:
- **Hidden states** = intended strokes
- **Observations** = typed strokes
- **Emission probabilities** = error model (key-distance based)
- **Transition probabilities** = n-gram LM over strokes

Viterbi decoding finds the most probable intended sequence given the observed (possibly erroneous) one.
Conceptually identical to the noisy channel model but framed as sequence decoding from the start.

**Best for**: offline correction of a full steno transcript.

### 12e. Conditional Random Field (CRF)

Frame correction as sequence labeling: a CRF reads the stroke sequence and labels each stroke as
CORRECT or MISTROKE, then a separate lookup applies the fix. This is simpler than seq2seq and works
well when you have even a modest amount of labeled error data (~10k–100k examples).

**Best for**: when you have some real error labels and want a trainable, interpretable model.

### 12f. Rule-Based Lookup Table

A hand-curated table of common (wrong_stroke → intended_stroke) pairs, possibly conditioned on
preceding context. Plover users already maintain "fingerspelling override" dictionaries that are
exactly this. For the top 50–100 most common personal mistroke patterns, a lookup table plus Hamming
distance fallback is surprisingly competitive.

**Best for**: personal use where you know your own error patterns; near-zero latency.

---

## 13. Recommended Approach for Stroke Correction

Given all of the above, the pragmatic recommendation is a **layered pipeline**:

```
Input strokes
     │
     ▼
[1] Canonicalize key order
     │
     ▼
[2] Dictionary lookup — if found, pass through unchanged
     │
     ▼  (only for strokes NOT in dictionary)
[3] Hamming-distance nearest-neighbor (catches ~70% of mechanical errors)
     │
     ▼  (ambiguous cases or valid-but-wrong strokes)
[4] Noisy channel decoder with KenLM n-gram LM (handles context)
     │
     ▼  (optional, only if you have writer-specific data)
[5] Fine-tuned small transformer re-ranker (handles brief disambiguation)
```

Start at layer 2 and only add layers when the previous layer's error rate stops improving. Most
projects will find layers 2–4 sufficient and never need a neural model at all.

---

## 14. The Multi-Stroke Similarity Problem

### Why Hamming Distance Breaks Down

Hamming distance on stroke bit vectors is stroke-count-sensitive by design. It completely fails when
comparing outlines with different numbers of strokes that encode the same phonetic content:

```
SEL/LEC/SHUN/S  →  4 bit vectors, ~50 total bits set
SLECTIONZ       →  1 bit vector,  ~10 bits set

Hamming distance = enormous   ✗  incorrectly "far apart"
```

Yet both outlines are perfectly reasonable ways to write "selections." A useful similarity metric must
treat them as close. Hamming distance on raw bits is the wrong space entirely.

### The Right Similarity Space: Phonemes

The fix is to flatten any multi-stroke outline into its **phoneme sequence** first, then compare in
phoneme space using edit distance:

```
SEL/LEC/SHUN/S  →  /s ɛ l ɛ k ʃ ə n z/
SLECTIONZ       →  /s l ɛ k ʃ ə n z/

Edit distance = 2   ✓  correctly "close"
```

This works regardless of how many strokes were used, because stroke boundaries are erased during the
phoneme flattening step.

### Implementation

You need two things: a steno-key-to-phoneme mapping table, and standard string edit distance. The
mapping is just a ~30-entry lookup table derived from your steno theory:

```python
# Left-hand keys → onset phonemes
LEFT = {
    'S': 's', 'T': 't', 'K': 'k', 'P': 'p', 'W': 'w',
    'H': 'h', 'R': 'r', 'PW': 'b', 'HR': 'l', 'KP': 'x',
    'TK': 'd', 'TP': 'f', 'SKWR': 'j', 'SH': 'ʃ', ...
}
# Vowel keys → nucleus phonemes
VOWELS = {
    'A': 'æ', 'O': 'ɒ', 'E': 'ɛ', 'U': 'ʌ', 'AO': 'uː',
    'AE': 'eɪ', 'OE': 'oʊ', 'EU': 'ɪ', ...
}
# Right-hand keys → coda phonemes
RIGHT = {
    '-F': 'f', '-P': 'p', '-R': 'r', '-B': 'b', '-L': 'l',
    '-G': 'g', '-T': 't', '-S': 's', '-D': 'd', '-Z': 'z',
    '-PB': 'n', '-PL': 'm', '-BG': 'k', '-GS': 'ʃ', ...
}

def outline_to_phonemes(outline):
    phonemes = []
    for stroke in outline.split('/'):
        phonemes += stroke_to_phonemes(stroke)  # split into left/vowel/right, map each
    return phonemes

def closest_matches(unknown_outline, dictionary, top_n=5):
    target = outline_to_phonemes(unknown_outline)
    scored = [
        (edit_distance(target, outline_to_phonemes(entry)), entry, translation)
        for entry, translation in dictionary.items()
    ]
    return sorted(scored)[:top_n]
```

### The Tricky Part: Compound Chords and Position-Dependence

A steno key means different things depending on where it appears in the stroke. Left-side `S` is an
onset /s/; right-side `-S` is a coda /s/ or /z/. The `*` (asterisk/inversion) key can alter the
phoneme of the entire stroke. When flattening a stroke to phonemes, you must split it into its
left-hand consonants, vowel cluster, and right-hand consonants first, then map each group.

### What Phoneme Distance Does Not Solve: Briefs

Briefs are phonetically **arbitrary by design** — `KP-BG` → "except" has no phonetic relationship to
the word it produces. No phoneme-space metric will find a correct near-miss for a brief, because the
brief's flattened phoneme sequence bears no resemblance to the target word.

For briefs, Hamming distance on raw bits is actually the right signal — you want to find strokes that
use nearly the same keys, regardless of what those keys phonetically encode.

### Recommended Blended Scoring

Combine both signals to cover the full range of outline types:

```python
score = 0.7 * phoneme_edit_distance + 0.3 * hamming_distance_normalized
```

| Outline type | Dominant signal |
|---|---|
| Multi-stroke phonetic outline | Phoneme edit distance |
| Single-stroke phonetic stroke | Both contribute equally |
| Brief (arbitrary chord) | Hamming distance |

Phoneme edit distance handles the stroke-count problem. Hamming distance as a secondary signal
rescues brief near-misses. Together they cover both cases without needing any training data.
