use std::collections::VecDeque;

use seagull::{Dictionary, JsonDictionary, Key, Outline, Stroke};

/// States for the search mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchStateEnum {
    /// Not in search mode
    Inactive,
    /// Actively searching, accumulating typed text
    Active(String),
    /// Showing the result of a lookup
    ShowingResult(String),
}

/// Check if a character is valid for search input (English keyboard characters)
pub fn is_search_key(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == ' ' || ".,!?;:'\"-".contains(ch)
}

/// A word that has been translated and is waiting in the committed queue.
#[derive(Debug, Clone)]
pub struct CommittedWord {
    pub word: String,
    pub outline: Outline,
}

/// Actions the engine should take after a stroke is pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferAction {
    /// Just update the preedit display (new pending strokes, no commit).
    UpdatePreedit,
    /// Commit a new word and update preedit.
    CommitAndPreedit,
    /// Flush all buffered content (committed words + pending strokes) to the application.
    FlushAll { flushed: String },
    /// Send a Space keypress (SP on empty buffer).
    SendSpace,
    /// Send an Enter keypress (R-R on empty buffer).
    SendEnter,
    /// Send a Backspace keypress (* on empty buffer).
    SendBackspace,
    /// Nothing changed (e.g. undo on empty buffer).
    Noop,
}

/// Buffers steno strokes, looks up translations, manages committed words.
pub struct StrokeBuffer {
    /// Pending strokes not yet translated.
    strokes: Vec<Stroke>,
    /// Words that have been translated but not yet flushed to the application.
    committed: VecDeque<CommittedWord>,
    /// Dictionary for looking up outlines.
    dictionary: JsonDictionary,
}

impl StrokeBuffer {
    pub fn new(dictionary: JsonDictionary) -> Self {
        Self {
            strokes: Vec::new(),
            committed: VecDeque::new(),
            dictionary,
        }
    }

    /// Check whether a stroke is *exactly* SP with no other keys.
    fn is_sp_only(stroke: Stroke) -> bool {
        stroke == Stroke::new(&[Key::LeftS, Key::LeftP])
    }

    /// Check whether a stroke is *exactly* R-R with no other keys.
    fn is_rr_only(stroke: Stroke) -> bool {
        stroke == Stroke::new(&[Key::LeftR, Key::RightR])
    }

    /// Check whether a stroke is *exactly* H*F (capitalize previous word).
    fn is_hstarf_only(stroke: Stroke) -> bool {
        stroke == Stroke::new(&[Key::LeftH, Key::MiddleStar, Key::RightF])
    }

    /// Capitalize the first character of a string.
    fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        }
    }

    /// Collect all buffered content into a single string and clear the buffer.
    fn flush_all(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for cw in self.committed.drain(..) {
            parts.push(cw.word);
        }
        if !self.strokes.is_empty() {
            let outline = Outline::from(self.strokes.as_slice());
            parts.push(outline.extended());
            self.strokes.clear();
        }
        parts.join(" ")
    }

    /// Clear the entire buffer (committed words and pending strokes).
    pub fn clear(&mut self) {
        self.strokes.clear();
        self.committed.clear();
    }

    /// Push a stroke into the buffer. Returns what the engine should do.
    pub fn push_stroke(&mut self, stroke: Stroke) -> BufferAction {
        if stroke == Stroke::star() {
            return self.undo();
        }

        // H*F alone: capitalize the previous (most recently committed) word.
        // Only works if there are no pending strokes.
        if Self::is_hstarf_only(stroke) {
            if self.strokes.is_empty() && !self.committed.is_empty() {
                if let Some(cw) = self.committed.back_mut() {
                    cw.word = Self::capitalize(&cw.word);
                    return BufferAction::CommitAndPreedit;
                }
            }
            return BufferAction::Noop;
        }

        if Self::is_sp_only(stroke) {
            let flushed = self.flush_all();
            if flushed.is_empty() {
                return BufferAction::SendSpace;
            }
            return BufferAction::FlushAll { flushed };
        }

        // R-R alone: flush all buffered content, or send Enter if empty.
        if Self::is_rr_only(stroke) {
            let flushed = self.flush_all();
            if flushed.is_empty() {
                return BufferAction::SendEnter;
            }
            return BufferAction::FlushAll { flushed };
        }

        self.strokes.push(stroke);

        // Try to look up the current pending strokes as an outline.
        let outline = Outline::from(self.strokes.as_slice());

        // 1. Check if combining with previous committed words forms a longer match
        if let Some((consume_count, word, combined_outline)) = self.find_longest_match(&outline) {
            // Check if any of the consumed words were capitalized
            let was_capitalized = (0..consume_count).any(|i| {
                let idx = self.committed.len() - 1 - i;
                let word_str = &self.committed[idx].word;
                // A very simple heuristic: if the first character is uppercase, it was capitalized.
                // Or we can just check if the very first consumed word is capitalized.
                // Let's check if the first consumed word (the oldest one in this match) is capitalized.
                if i == consume_count - 1 {
                    word_str.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                } else {
                    false
                }
            });

            let final_word = if was_capitalized {
                Self::capitalize(&word)
            } else {
                word.to_owned()
            };

            // Remove the consumed words
            for _ in 0..consume_count {
                self.committed.pop_back();
            }

            let committed_word = CommittedWord {
                word: final_word,
                outline: combined_outline,
            };

            self.strokes.clear();
            self.committed.push_back(committed_word);
            return BufferAction::CommitAndPreedit;
        }

        // 2. If no longer match, check the current strokes alone
        if let Some(word) = self.dictionary.lookup(outline.clone()) {
            let committed_word = CommittedWord {
                word: word.to_owned(),
                outline: outline.clone(),
            };
            self.strokes.clear();

            self.committed.push_back(committed_word);
            BufferAction::CommitAndPreedit
        } else {
            BufferAction::UpdatePreedit
        }
    }
    /// Check if combining previous committed words with the pending outline forms a longer word.
    /// Returns the number of committed words to consume, the new word, and the combined outline.
    fn find_longest_match(&self, pending_outline: &Outline) -> Option<(usize, String, Outline)> {
        let mut longest_match = None;
        let mut current_outline = pending_outline.clone();

        // Iterate backwards through committed words
        for (i, cw) in self.committed.iter().rev().enumerate() {
            // Combine outline by prefixing the committed word's outline
            let combined = cw.outline.clone() / current_outline;

            // Check if this combined outline is in the dictionary
            if let Some(word) = self.dictionary.lookup(combined.clone()) {
                longest_match = Some((i + 1, word.to_owned(), combined.clone()));
            }

            current_outline = combined;
        }

        longest_match
    }


    /// Undo the last stroke or decompose the last committed word.
    fn undo(&mut self) -> BufferAction {
        if !self.strokes.is_empty() {
            self.strokes.pop();
            BufferAction::UpdatePreedit
        } else if self.committed.is_empty() {
            BufferAction::SendBackspace
        } else if let Some(last_committed) = self.committed.pop_back() {
            // Decompose: restore all strokes of the last word minus the final one.
            let outline_strokes = last_committed.outline.strokes().to_vec();
            if outline_strokes.len() > 1 {
                self.strokes = outline_strokes[..outline_strokes.len() - 1].to_vec();
            }
            // If it was a single-stroke word, strokes stays empty.
            BufferAction::UpdatePreedit
        } else {
            BufferAction::Noop
        }
    }

    /// Build the preedit string for display.
    /// Format: `{committed1} {committed2} STROKE1/STROKE2`
    pub fn preedit_string(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        for cw in &self.committed {
            parts.push(cw.word.clone());
        }

        if !self.strokes.is_empty() {
            let outline = Outline::from(self.strokes.as_slice());
            parts.push(outline.extended());
        }

        parts.join(" ")
    }

    /// Number of committed words currently in the buffer.
    pub fn committed_len(&self) -> usize {
        self.committed.len()
    }

    /// Number of pending strokes.
    pub fn pending_len(&self) -> usize {
        self.strokes.len()
    }

    /// The byte offset where pending strokes begin in the preedit string.
    /// Used for setting underline attributes.
    pub fn committed_preedit_len(&self) -> usize {
        let mut len = 0;
        for (i, cw) in self.committed.iter().enumerate() {
            if i > 0 {
                len += 1; // space separator
            }
            len += cw.word.len();
        }
        len
    }

    /// Reverse lookup: find the outline for a given word.
    /// Returns None if the word is not in the dictionary.
    pub fn reverse_lookup_word(&self, word: &str) -> Option<Outline> {
        self.dictionary.reverse_lookup(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Build a small test dictionary from outline strings → words.
    fn test_dictionary(entries: &[(&str, &str)]) -> JsonDictionary {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pairs: Vec<String> = entries
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", k, v))
            .collect();
        let json_str = format!("{{{}}}", pairs.join(","));

        let tmp = std::env::temp_dir().join(format!("seagull_test_dict_{}.json", id));
        std::fs::write(&tmp, &json_str).unwrap();
        JsonDictionary::load_from_file(&tmp).unwrap()
    }

    #[test]
    fn test_push_single_stroke_word() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        let stroke = Stroke::try_from_string("KAT").unwrap();
        let action = buf.push_stroke(stroke);
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.committed_len(), 1);
        assert_eq!(buf.preedit_string(), "cat");
    }

    #[test]
    fn test_push_unknown_stroke() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        let stroke = Stroke::try_from_string("SKWR").unwrap();
        let action = buf.push_stroke(stroke);
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 1);
    }

    #[test]
    fn test_undo_pending_stroke() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());
        assert_eq!(buf.pending_len(), 1);

        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_undo_committed_word() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(buf.committed_len(), 1);

        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_undo_multistroke_committed_word() {
        let dict = test_dictionary(&[("KAT/ER", "cater")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        let action = buf.push_stroke(Stroke::try_from_string("ER").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.committed_len(), 1);

        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 1);
    }

    #[test]
    fn test_undo_empty_sends_backspace() {
        let dict = test_dictionary(&[]);
        let mut buf = StrokeBuffer::new(dict);

        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::SendBackspace);
    }

    #[test]
    fn test_preedit_string_mixed() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());

        let preedit = buf.preedit_string();
        assert!(preedit.starts_with("cat "), "preedit was: {preedit}");
    }

    #[test]
    fn test_rr_flushes_all() {
        let dict = test_dictionary(&[("KAT", "cat"), ("TKOG", "dog")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        buf.push_stroke(Stroke::try_from_string("TKOG").unwrap());
        assert_eq!(buf.committed_len(), 2);

        let rr = Stroke::new(&[Key::LeftR, Key::RightR]);
        let action = buf.push_stroke(rr);
        match action {
            BufferAction::FlushAll { ref flushed } => {
                assert_eq!(flushed, "cat dog");
            }
            _ => panic!("Expected FlushAll, got {:?}", action),
        }
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_rr_flushes_pending_too() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());

        let rr = Stroke::new(&[Key::LeftR, Key::RightR]);
        let action = buf.push_stroke(rr);
        match action {
            BufferAction::FlushAll { ref flushed } => {
                assert!(flushed.starts_with("cat "), "flushed was: {flushed}");
            }
            _ => panic!("Expected FlushAll, got {:?}", action),
        }
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_rr_on_empty_sends_enter() {
        let dict = test_dictionary(&[]);
        let mut buf = StrokeBuffer::new(dict);

        let rr = Stroke::new(&[Key::LeftR, Key::RightR]);
        let action = buf.push_stroke(rr);
        assert_eq!(action, BufferAction::SendEnter);
    }

    #[test]
    fn test_clear_buffer() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());
        assert_eq!(buf.committed_len(), 1);
        assert_eq!(buf.pending_len(), 1);

        buf.clear();
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 0);
        assert_eq!(buf.preedit_string(), "");
    }


    #[test]
    fn test_longest_word_wins_simple() {
        let dict = test_dictionary(&[
            ("KAT", "cat"),
            ("KAT/ER", "cater"),
        ]);
        let mut buf = StrokeBuffer::new(dict);

        // First stroke -> commits "cat"
        let action = buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "cat");
        assert_eq!(buf.committed_len(), 1);

        // Second stroke -> decomposes "cat", recombines into "cater"
        let action = buf.push_stroke(Stroke::try_from_string("ER").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "cater");
        assert_eq!(buf.committed_len(), 1);
    }

    #[test]
    fn test_longest_word_wins_multi_word() {
        let dict = test_dictionary(&[
            ("S", "saw"),
            ("KAT", "cat"),
            ("S/KAT/ER", "scaterer"),
        ]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("S").unwrap());
        assert_eq!(buf.preedit_string(), "saw");

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(buf.preedit_string(), "saw cat");
        assert_eq!(buf.committed_len(), 2);

        // This stroke should trigger a decomposition of both previous words
        let action = buf.push_stroke(Stroke::try_from_string("ER").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "scaterer");
        assert_eq!(buf.committed_len(), 1);
    }

    #[test]
    fn test_longest_word_wins_no_match() {
        let dict = test_dictionary(&[
            ("KAT", "cat"),
            ("KAT/S", "cats"),
        ]);
        let mut buf = StrokeBuffer::new(dict);

        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());

        // This makes "cats"
        buf.push_stroke(Stroke::try_from_string("S").unwrap());
        assert_eq!(buf.preedit_string(), "cats");
        assert_eq!(buf.committed_len(), 1);

        // Another 'S' -> not in dict. Should stay pending since "S" alone is not in dict here.
        let action = buf.push_stroke(Stroke::try_from_string("S").unwrap());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.preedit_string(), "cats S");
        assert_eq!(buf.committed_len(), 1);
        assert_eq!(buf.pending_len(), 1);
    }

    #[test]
    fn test_longest_word_wins_capitalization() {
        let dict = test_dictionary(&[
            ("KAT", "cat"),
            ("KAT/ER", "cater"),
        ]);
        let mut buf = StrokeBuffer::new(dict);

        // Commit "cat"
        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(buf.preedit_string(), "cat");

        // H*F to capitalize the previous word (cat -> Cat)
        buf.push_stroke(Stroke::new(&[Key::LeftH, Key::MiddleStar, Key::RightF]));
        assert_eq!(buf.preedit_string(), "Cat");

        // Add "ER" to combine into "cater" - the capitalization of the original
        // consumed word ("Cat") should be preserved in the recombination
        buf.push_stroke(Stroke::try_from_string("ER").unwrap());
        assert_eq!(buf.preedit_string(), "Cater");
    }

    #[test]
    fn test_hstarf_capitalizes_previous_word() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict);

        // First word: "cat"
        let action = buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "cat");

        // H*F alone: capitalize the previous word
        let hstarf = Stroke::new(&[Key::LeftH, Key::MiddleStar, Key::RightF]);
        let action = buf.push_stroke(hstarf);
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "Cat");

        // Next word: "cat" (not affected by H*F)
        let action = buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.preedit_string(), "Cat cat");
    }

    #[test]
    fn test_reverse_lookup_word_valid() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let buf = StrokeBuffer::new(dict);

        let result = buf.reverse_lookup_word("cat");
        assert!(result.is_some());
        let outline = result.unwrap();
        assert_eq!(outline.to_string(), "KAT");
    }

    #[test]
    fn test_reverse_lookup_word_invalid() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let buf = StrokeBuffer::new(dict);

        let result = buf.reverse_lookup_word("xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_is_search_key_alphanumeric() {
        assert!(is_search_key('a'));
        assert!(is_search_key('Z'));
        assert!(is_search_key('0'));
        assert!(is_search_key('9'));
    }

    #[test]
    fn test_is_search_key_punctuation() {
        assert!(is_search_key(' '));
        assert!(is_search_key('.'));
        assert!(is_search_key(','));
        assert!(is_search_key('!'));
        assert!(is_search_key('?'));
    }

    #[test]
    fn test_is_search_key_invalid() {
        assert!(!is_search_key('@'));
        assert!(!is_search_key('#'));
        assert!(!is_search_key('€'));
    }

    #[test]
    fn test_search_state_enum_variations() {
        let state1 = SearchStateEnum::Inactive;
        assert!(matches!(state1, SearchStateEnum::Inactive));

        let state2 = SearchStateEnum::Active("hello".to_string());
        assert!(matches!(state2, SearchStateEnum::Active(ref s) if s == "hello"));

        let state3 = SearchStateEnum::ShowingResult("world".to_string());
        assert!(matches!(state3, SearchStateEnum::ShowingResult(ref s) if s == "world"));
    }
}
