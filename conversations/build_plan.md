# Seagull IME: Longest Word Wins Build Plan

## Overview
Implement a feature where the IME decomposes previously committed words and recombines them with new strokes to form longer words if available. The rule is: **the longest word you can spell with a sequence of strokes wins**.

## Key Design Considerations

1. **When to trigger**: After each new stroke is added, check if:
   - The pending strokes form a word (existing behavior)
   - Additionally, check if combining the outlines of previously committed words with the pending strokes creates an even longer valid word in the dictionary.

2. **How to find the longest word**:
   - Start from the most recently committed word and work backwards.
   - Try combining the outlines of 1, 2, 3, ... N committed words with the current pending strokes.
   - Find the longest valid word in the dictionary (i.e., the one that consumes the most committed words).
   - If a longer word is found, replace the N committed words and the pending strokes with the single new longer word.

3. **Data Flow Changes in `StrokeBuffer::push_stroke`**:
   - Calculate the new outline for the pending strokes.
   - Iterate backwards through `self.committed`.
   - For each step back (let's say we go back `i` words), construct a combined outline: `committed[len - i].outline / ... / committed[len - 1].outline / pending_outline`.
   - Lookup this combined outline in the dictionary.
   - Keep track of the match that goes back the furthest (largest `i`).
   - If a match is found that includes at least one committed word:
     - Remove those `i` words from the end of `self.committed`.
     - Clear `self.strokes`.
     - Add the new, longer word to `self.committed` (handling capitalization appropriately).
     - Return `BufferAction::CommitAndPreedit` (or potentially a new action if the UI needs to know about the deletion, though `CommitAndPreedit` currently implies a full redraw of the preedit state, which might be sufficient).
   - If no longer match is found, fallback to the existing behavior:
     - Check if the pending strokes alone form a word.
     - If yes, commit it.
     - If no, keep them pending (`BufferAction::UpdatePreedit`).

## Implementation Tasks

### 1. Add Helper Methods to `StrokeBuffer`
- Create a method to check for the longest matching outline by combining committed words and pending strokes.
  - `fn find_longest_match(&self, pending_outline: &Outline) -> Option<(usize, String, Outline)>`
  - Returns `(number_of_committed_words_to_consume, translated_word, combined_outline)`.

### 2. Modify `StrokeBuffer::push_stroke`
- Integrate the `find_longest_match` logic.
- Handle the removal of the consumed committed words.
- Ensure capitalization rules (like `capitalize_next`) are applied correctly to the newly formed longer word. If the very first word being consumed was capitalized, the new longer word should probably also be capitalized.

### 3. Update UI/Action Enums (If necessary)
- Check if `BufferAction` needs modification. Currently, the UI likely clears its preedit display and redraws it based on the buffer's state. If so, simply popping from `self.committed` and pushing the new word, then returning `CommitAndPreedit`, might just work seamlessly.

### 4. Write Comprehensive Unit Tests
- Add tests in `ime/src/buffer.rs`:
  - **Simple De-composition**: Commit "cat" (`KAT`), then input `ER`. Should result in "cater" (`KAT/ER`).
  - **Multi-word De-composition**: Commit "saw" (`S`), commit "cat" (`KAT`), input `ER`. If `S/KAT/ER` is "scaterer", it should consume both "saw" and "cat".
  - **No Longer Match**: Commit "cat" (`KAT`), input `S`. If `KAT/S` is "cats" but `S/KAT/S` is nothing, it should only consume "cat".
  - **Capitalization Preservation**: If "Cat" (capitalized) is committed, and `ER` is added, it should become "Cater".
  - **Undo behavior**: Check how `Stroke::star()` interacts. (Currently, undo pops the last stroke. If we recompose, undoing might need to just pop the last stroke of the recomposed outline and re-evaluate. This might be tricky and needs careful consideration, but the current `Machine` logic in `seagull/src/lib.rs` suggests undo is just outline-based).

### 5. Review `seagull/src/game.rs`
- The typing game also has a `push_stroke` equivalent.
- Decide if the game needs this behavior. Given it's a typing test, it might be confusing if words randomly combine and change the user's past mistakes/successes. It might be better to leave the game logic as strict word-by-word, or update it carefully to match.

## Open Questions / Edge Cases to Consider
- **Performance**: Iterating backwards through all committed words on every stroke could be O(N^2) or worse if the buffer is huge. In practice, `self.committed` shouldn't be massive before it gets flushed, but we might want a limit (e.g., only look back max 5-10 words).
- **Undo (*)**: If the user types `KAT` -> "cat", then `ER` -> "cater", and then hits `*` (Undo), what should happen? The current `buffer.undo()` logic pops from `self.strokes`, or if empty, pops from `self.committed` and puts its outline back into `self.strokes` minus the last stroke. We need to verify `buffer.undo()` correctly handles decomposed/recomposed words.
