use std::collections::VecDeque;

use seagull::{Dictionary, JsonDictionary, Outline, Stroke};

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
    /// Flush the oldest committed word to the application, commit a new word, and update preedit.
    FlushAndCommitAndPreedit { flushed: String },
    /// Nothing changed (e.g. undo on empty buffer).
    Noop,
}

/// Buffers steno strokes, looks up translations, manages committed words.
pub struct StrokeBuffer {
    /// Pending strokes not yet translated.
    strokes: Vec<Stroke>,
    /// Words that have been translated but not yet flushed to the application.
    committed: VecDeque<CommittedWord>,
    /// Maximum number of committed word slots before flushing.
    max_slots: usize,
    /// Dictionary for looking up outlines.
    dictionary: JsonDictionary,
}

impl StrokeBuffer {
    pub fn new(dictionary: JsonDictionary, max_slots: usize) -> Self {
        Self {
            strokes: Vec::new(),
            committed: VecDeque::new(),
            max_slots,
            dictionary,
        }
    }

    /// Push a stroke into the buffer. Returns what the engine should do.
    pub fn push_stroke(&mut self, stroke: Stroke) -> BufferAction {
        if stroke == Stroke::star() {
            return self.undo();
        }

        self.strokes.push(stroke);

        // Try to look up the current pending strokes as an outline.
        let outline = Outline::from(self.strokes.as_slice());
        if let Some(word) = self.dictionary.lookup(outline.clone()) {
            let word = word.to_owned();
            let committed_word = CommittedWord {
                word,
                outline: outline.clone(),
            };
            self.strokes.clear();

            if self.committed.len() >= self.max_slots {
                // Buffer full — flush oldest.
                let flushed = self.flush_oldest();
                self.committed.push_back(committed_word);
                BufferAction::FlushAndCommitAndPreedit { flushed }
            } else {
                self.committed.push_back(committed_word);
                BufferAction::CommitAndPreedit
            }
        } else {
            BufferAction::UpdatePreedit
        }
    }

    /// Undo the last stroke or decompose the last committed word.
    fn undo(&mut self) -> BufferAction {
        if !self.strokes.is_empty() {
            self.strokes.pop();
            BufferAction::UpdatePreedit
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

    /// Remove and return the oldest committed word.
    fn flush_oldest(&mut self) -> String {
        self.committed
            .pop_front()
            .expect("flush_oldest called with no committed words")
            .word
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
        let mut buf = StrokeBuffer::new(dict, 5);

        let stroke = Stroke::try_from_string("KAT").unwrap();
        let action = buf.push_stroke(stroke);
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.committed_len(), 1);
        assert_eq!(buf.preedit_string(), "cat");
    }

    #[test]
    fn test_push_unknown_stroke() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict, 5);

        let stroke = Stroke::try_from_string("SKWR").unwrap();
        let action = buf.push_stroke(stroke);
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 1);
    }

    #[test]
    fn test_undo_pending_stroke() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict, 5);

        // Push an unknown stroke
        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());
        assert_eq!(buf.pending_len(), 1);

        // Undo it with star
        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_undo_committed_word() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict, 5);

        // Commit "cat"
        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(buf.committed_len(), 1);

        // Undo — should decompose. KAT is single stroke, so strokes go empty.
        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn test_undo_multistroke_committed_word() {
        let dict = test_dictionary(&[("KAT/ER", "cater")]);
        let mut buf = StrokeBuffer::new(dict, 5);

        // Two strokes to get "cater"
        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        let action = buf.push_stroke(Stroke::try_from_string("ER").unwrap());
        assert_eq!(action, BufferAction::CommitAndPreedit);
        assert_eq!(buf.committed_len(), 1);

        // Undo — should restore all but last stroke
        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::UpdatePreedit);
        assert_eq!(buf.committed_len(), 0);
        assert_eq!(buf.pending_len(), 1); // "KAT" restored as pending
    }

    #[test]
    fn test_undo_empty_is_noop() {
        let dict = test_dictionary(&[]);
        let mut buf = StrokeBuffer::new(dict, 5);

        let action = buf.push_stroke(Stroke::star());
        assert_eq!(action, BufferAction::Noop);
    }

    #[test]
    fn test_buffer_full_flushes_oldest() {
        let dict = test_dictionary(&[("KAT", "cat"), ("TKOG", "dog")]);
        let mut buf = StrokeBuffer::new(dict, 1); // max 1 committed slot

        // Commit "cat"
        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        assert_eq!(buf.committed_len(), 1);

        // Commit "dog" — should flush "cat" first
        let action = buf.push_stroke(Stroke::try_from_string("TKOG").unwrap());
        match action {
            BufferAction::FlushAndCommitAndPreedit { ref flushed } => {
                assert_eq!(flushed, "cat");
            }
            _ => panic!("Expected FlushAndCommitAndPreedit, got {:?}", action),
        }
        assert_eq!(buf.committed_len(), 1);
        assert_eq!(buf.preedit_string(), "dog");
    }

    #[test]
    fn test_preedit_string_mixed() {
        let dict = test_dictionary(&[("KAT", "cat")]);
        let mut buf = StrokeBuffer::new(dict, 5);

        // Commit "cat"
        buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
        // Add unknown stroke
        buf.push_stroke(Stroke::try_from_string("SKWR").unwrap());

        let preedit = buf.preedit_string();
        // Should be "cat" + space + extended notation of SKWR
        assert!(preedit.starts_with("cat "), "preedit was: {preedit}");
    }
}
