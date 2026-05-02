# Search Feature Code Walkthrough

## Feature Flow: Control+S Activation

### 1. User presses Control+S
- Steno device sends `Keycode` with control flag and LeftS stroke
- Received in main.rs event loop (line 272)

### 2. Main.rs Event Loop Detection (lines 287-294)
```rust
if keycode.is_control() && stroke == Stroke::new(&[Key::LeftS]) {
    log!(logger, "  Control+S: activating search mode");
    if let Err(e) = engine.show_search().await {
        log!(logger, "  ERROR activating search: {e}");
    }
    continue;  // Skip to next iteration
}
```
- Calls `engine.show_search()` which:
  - Sets search_state to `Active("")`
  - Emits "SEARCH: " via auxiliary text display

### 3. Normal Strokes Skip (lines 325-329)
```rust
{
    let search = search_state.lock().await;
    if matches!(*search, SearchStateEnum::Active(_)) {
        log!(logger, "  Skipping stroke while search is active");
        continue;
    }
}
```
- While search is Active, steno strokes are ignored
- Allows keyboard input to take priority

## Keyboard Input Handling

### 4. IBus Keyboard Event (engine.rs process_key_event)
- Line 240: Check if search mode Active
- Line 244: Call `handle_search_key_event(keyval)`
- Returns true (consume key) for valid search input

### 5. Search Key Handler (engine.rs, lines 269-334)
```rust
async fn handle_search_key_event(&self, keyval: u32) -> bool
```
Handles:
- **ESC (0xFF1B)**: Close search, hide auxiliary text
- **Backspace (0xFF08)**: Delete last character, update display
- **Enter (0xFF0D)**: Perform lookup, show result
- **Regular chars**: Validate with `is_search_key()`, add to buffer

### 6. Character Validation (buffer.rs, line 17-19)
```rust
pub fn is_search_key(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == ' ' || ".,!?;:'\"-".contains(ch)
}
```

## Dictionary Lookup

### 7. Reverse Lookup (buffer.rs, line 271-273)
```rust
pub fn reverse_lookup_word(&self, word: &str) -> Option<Outline> {
    self.dictionary.reverse_lookup(word)
}
```
- Delegates to seagull library's Dictionary trait
- Returns Outline if word found, None otherwise

### 8. Lookup Display (engine.rs, perform_lookup method)
- If found: Display "[outline] [word]"  (e.g., "KAT cat")
- If not found: Display "NOT FOUND [word]"

## State Management

### 9. SearchStateEnum (buffer.rs, lines 5-14)
Three states:
- **Inactive**: Not searching, normal steno mode
- **Active(String)**: User typing search query
- **ShowingResult(String)**: Result displayed, waiting for user action

## Key Integration Points

1. **buffer.rs**: State definition + reverse lookup
2. **engine.rs**: Search methods + keyboard handler
3. **main.rs**: Control+S detection + stroke skipping + initialization

## Result Display
- Uses existing `emit_auxiliary_text()` function
- Shows dynamically as user types
- Matches HINT popup behavior
