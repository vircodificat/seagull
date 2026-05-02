# `StrokeBuffer::reprocess()` — Step-by-Step

The goal of `reprocess()` is: after a new stroke is appended to the buffer,
re-run the longest-match dictionary search over the tail of the buffer to turn
raw strokes into committed words or punctuation.

---

## Step 1 — Find the reprocessing start point

```rust
let rightmost_pos = self.buffer.iter().rposition(|el| {
    matches!(el, Element::CommittedWord(_) | Element::Punctuation(_))
});

let reprocess_from = match rightmost_pos {
    Some(0) => 0,
    Some(pos) => pos - 1,
    None => 0,
};
```

- **`rightmost_pos`** — the index of the last `CommittedWord` or `Punctuation` element already in the buffer (searching from the right).
- **`reprocess_from`** — where to begin re-matching. It goes *one step back* from the rightmost committed element, because a new stroke might combine with the previous word (e.g., a multi-stroke outline whose final stroke was just added). If there are no committed elements at all, it starts from index `0`.

---

## Step 2 — Determine the capitalization state before that point

```rust
let mut current_cap_state = false;
for i in (0..reprocess_from).rev() {
    if let Element::Punctuation(p) = &self.buffer[i] {
        current_cap_state = p.caps_after;
        break;
    }
}
```

- **`current_cap_state`** — whether the word at the reprocessing start point should be capitalized. Determined by scanning *backwards* through elements before `reprocess_from` to find the nearest punctuation. If that punctuation has `caps_after = true` (sentence-ending like `.`, `!`, `?`), the first re-matched word will be capitalized.

---

## Step 3 — Extract elements to reprocess

```rust
let elements_to_reprocess: Vec<Element> = self.buffer.split_off(reprocess_from).into();
```

- **`elements_to_reprocess`** — everything from `reprocess_from` to the end of the buffer is removed from `self.buffer` and collected here. These elements will be re-matched from scratch. `self.buffer` now only contains the "settled" prefix that will not be touched.

---

## Step 4 — Flatten elements into a stroke sequence

```rust
let mut all_strokes = Vec::new();
let mut capitalized_at: HashMap<usize, bool> = HashMap::new();
let mut stroke_index = 0;
for el in &elements_to_reprocess { ... }
```

- **`all_strokes`** — a flat `Vec<Stroke>` built by unrolling every element back to its raw strokes.
  - A `Stroke` element contributes itself.
  - A `CapsNext` element is re-encoded as the `H-F` stroke so it can participate in matching.
  - A `CommittedWord` contributes its `outline.strokes()`.
  - A `Punctuation` contributes its `outline.strokes()`.
- **`stroke_index`** — a running counter tracking which position in `all_strokes` the next element's strokes start at.
- **`capitalized_at`** — maps a stroke index → `true` for any previously committed word whose text started with an uppercase letter. Allows a re-match covering the same strokes to preserve explicit capitalization (e.g., via H*F).

---

## Step 5 — Greedy longest-match loop

```rust
let mut i = 0;
let mut should_capitalize = current_cap_state;
while i < all_strokes.len() {
    let mut best_match_len = 0;
    let mut best_match_element = None;

    for j in (i + 1)..=all_strokes.len() {
        let outline = Outline::from(&all_strokes[i..j]);
        // priority: sentence punct → inline punct → dictionary word
        ...
    }
}
```

- **`i`** — current position in `all_strokes`. Advances by `best_match_len` after each successful match.
- **`should_capitalize`** — whether the *next* matched word should start with a capital letter. Starts from `current_cap_state` and updated after each element: `true` after sentence-ending punctuation, `false` after inline punctuation or a regular word.
- **`j`** — the exclusive end of the slice being tested (`all_strokes[i..j]`). The inner loop tries every possible length, from 1 stroke up to the end of the list.
- **`outline`** — the `Outline` formed from `all_strokes[i..j]`, tested against punctuation rules and the dictionary.
- **`best_match_len`** — the number of strokes consumed by the best (longest) match found so far.
- **`best_match_element`** — the `Element` produced by the best match. Because the inner loop goes from short to long and always overwrites on success, the final value is the *longest* match.
- **`was_consumed_capitalized`** — looked up from `capitalized_at` at index `i`; `true` if the strokes at this position previously produced a capitalized word.

Match priority inside the inner loop:
1. Sentence-ending punctuation (hard-coded outlines, e.g. `TP-PL` → `.`)
2. Inline punctuation (hard-coded outlines, e.g. `KW-BG` → `,`)
3. Dictionary lookup — if the result is a marker like `{.}` it becomes `Punctuation`; otherwise a `CommittedWord`, capitalized if `should_capitalize || was_consumed_capitalized`.

---

## Step 6 — Commit the best match or fall back to raw strokes

```rust
if let Some(el) = best_match_element {
    ...
    self.buffer.push_back(el);
    i += best_match_len;
} else {
    for j in i..all_strokes.len() {
        self.buffer.push_back(Element::Stroke(all_strokes[j]));
    }
    break;
}
```

- **Match found:** push the `CommittedWord` or `Punctuation` onto `self.buffer`, advance `i`, update `should_capitalize`.
- **No match:** all remaining strokes (from `i` to the end) are pushed back as raw `Element::Stroke` items — these are the *pending* strokes shown in the preedit underline.

---

## Step 7 — Persist capitalization state

```rust
self.capitalize_next = should_capitalize;
```

- **`self.capitalize_next`** — stores the final capitalization flag so it is available across future strokes.

---

## Variable Summary

| Variable | Meaning |
|---|---|
| `rightmost_pos` | Index of the last committed word/punct in the existing buffer |
| `reprocess_from` | Index to slice the buffer at; one step before `rightmost_pos` |
| `current_cap_state` | Whether to capitalize at the reprocessing start, inherited from the nearest preceding punctuation |
| `elements_to_reprocess` | Elements removed from the buffer to be re-matched from scratch |
| `all_strokes` | All raw strokes from `elements_to_reprocess`, flattened |
| `stroke_index` | Running counter while building `all_strokes` |
| `capitalized_at` | Map: stroke-start-index → was-capitalized, preserves explicit capitalization across re-matches |
| `i` | Current read position in `all_strokes` |
| `should_capitalize` | Whether the next committed word should be uppercased |
| `j` | End of the stroke slice being tested in the inner loop |
| `outline` | `Outline` formed from `all_strokes[i..j]` |
| `best_match_len` | Stroke count of the longest match found so far |
| `best_match_element` | The resulting `Element` for the longest match |
| `was_consumed_capitalized` | Whether a prior committed word at this stroke position was capitalized |
