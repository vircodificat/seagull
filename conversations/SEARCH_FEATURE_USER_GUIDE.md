# Search Feature User Guide

## Quick Start

### Activating Search Mode
1. **Press Control+S** (hold Control, press S on steno machine)
2. Auxiliary text will display: `SEARCH: `
3. You're now in search mode

### Using Search Mode

#### Typing a Word
- Use keyboard to type English words
- Supported characters: a-z, A-Z, 0-9, space, and punctuation (.,!?;:'-")
- Auxiliary text updates in real-time: `SEARCH: hello`

#### Finding the Steno Outline
1. Type your word (e.g., "hello")
2. Press **Enter**
3. Auxiliary text will show one of:
   - **`HL/O hello`** ← Success! Shows the outline and word
   - **`NOT FOUND hello`** ← Word not in dictionary

#### Deleting Characters
- Press **Backspace** to delete the last character
- Auxiliary text updates: `SEARCH: hell` (removed 'o')
- Continue typing to re-add characters

#### Closing Search (Without Lookup)
- Press **Escape** to close search mode
- No dictionary lookup performed
- Returns to normal steno mode

#### Returning to Normal Steno
- After viewing result or pressing ESC
- Just start typing steno strokes normally
- Preedit will show pending strokes as usual

## Examples

### Example 1: Look up "cat"
```
Press Control+S
  → SEARCH: 

Type "cat"
  → SEARCH: cat

Press Enter
  → KAT cat
  
(Shows outline KAT maps to word "cat")
```

### Example 2: Search + Cancel
```
Press Control+S
  → SEARCH: 

Type "xyz"
  → SEARCH: xyz

Press Escape
  → (Back to normal steno mode)
```

### Example 3: Correct Typo
```
Type "helo" (typo)
  → SEARCH: helo

Press Backspace 4 times
  → SEARCH: 

Type "hello"
  → SEARCH: hello

Press Enter
  → HL/O hello
```

## Tips

1. **Exact Match Only**: Search looks for exact word matches in dictionary
2. **Case Sensitive**: "Hello" and "hello" may be treated differently
3. **No Multi-Word Lookups**: Search one word at a time
4. **Speed**: Lookups are instant - result appears immediately
5. **Steno Continues**: Other steno strokes ignored while searching (this is intentional)

## Troubleshooting

### Search Mode Won't Activate
- Ensure Control key on steno machine is held
- S key should be the ONLY key pressed
- Check that IME is focused

### Result Shows "NOT FOUND"
- Word may not be in dictionary
- Check spelling/capitalization
- Try variations of the word

### Can't Type Characters
- Only valid English keyboard characters accepted
- Try: letters, numbers, space, . , ! ? ; : ' " -
- Special characters (@, #, €) are not supported

### Auxiliary Text Not Showing
- Check that application supports IBus auxiliary text
- Some applications may not display it
- Try different text editor/input field

## Advanced Notes

### Dictionary Source
The search looks up words in the loaded Seagull IME dictionary.
Current dictionary file: `/home/vircodificat/projects/seagull/data/seagull.json`

### Outline Format
Outlines are displayed in Seagull notation:
- Single stroke: `KAT`
- Multi-stroke: `KAT/ER` (separated by /)
- Middle keys shown in middle: `KAT` vs `KAT/ER`

### Performance
- Searches happen instantly (dictionary is pre-loaded in memory)
- No network calls needed
- Works offline
