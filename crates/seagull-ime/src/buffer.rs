#[cfg(test)]
mod tests;

use log::info;
use std::collections::VecDeque;

use seagull::{Dictionary, JsonDictionary, Key, Outline, Stroke};

/// States for the search mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
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

/// A word that has been translated and is in the buffer.
#[derive(Debug, Clone)]
pub struct CommittedWord {
    pub word: String,
    pub outline: Outline,
    pub join_left: bool,
    pub join_right: bool,
}

/// Punctuation mark in the buffer.
#[derive(Debug, Clone)]
pub struct Punctuation {
    pub punct: String,
    pub outline: Outline,
    pub caps_next: bool,
}

/// A single element in the buffer: either a stroke, committed word, or punctuation.
#[derive(Debug, Clone)]
pub enum Element {
    Stroke(Stroke),
    CommittedWord(CommittedWord),
    Punctuation(Punctuation),
    CapsNext,
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

/// Buffers steno strokes, looks up translations, manages committed words and punctuation.
/// The buffer contains Elements (Stroke, CommittedWord, or Punctuation) that can be freely intermingled.
pub struct StrokeBuffer {
    /// Strokes, words, and punctuation that have not yet been flushed to the application.
    buffer: VecDeque<Element>,
    /// Dictionary for looking up outlines.
    dictionary: JsonDictionary,
}

impl StrokeBuffer {
    pub fn new(dictionary: JsonDictionary) -> Self {
        Self {
            buffer: VecDeque::new(),
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

    fn is_hf_only(stroke: Stroke) -> bool {
        stroke == Stroke::new(&[Key::LeftH, Key::RightF])
    }

    /// Insert a `CapsNext` element before the trailing run of pending strokes,
    /// or just before the most recently committed word/punctuation if none are pending.
    fn insert_caps_previous(&mut self) -> BufferAction {
        let mut insert_pos = self.buffer.len();
        while insert_pos > 0 && matches!(self.buffer[insert_pos - 1], Element::Stroke(_)) {
            insert_pos -= 1;
        }
        let had_pending_strokes = insert_pos < self.buffer.len();
        if had_pending_strokes {
            self.buffer.insert(insert_pos, Element::CapsNext);
            self.reprocess();
            BufferAction::CommitAndPreedit
        } else if insert_pos > 0 {
            self.buffer.insert(insert_pos - 1, Element::CapsNext);
            BufferAction::CommitAndPreedit
        } else {
            BufferAction::Noop
        }
    }

    /// If the outline is a sentence-ending punctuation marker, return a Punctuation struct.
    /// These attach without a preceding space and capitalize the next word.
    fn sentence_end_punct(outline: &Outline) -> Option<Punctuation> {
        let punct = match outline.to_string().as_str() {
            "TP-PL" => ".",
            "TP-BG" => "!",
            "KW-PL" => "?",
            _ => return None,
        };
        Some(Punctuation {
            punct: punct.to_string(),
            outline: outline.clone(),
            caps_next: true,
        })
    }

    /// If the outline is an inline punctuation marker, return a Punctuation struct.
    /// These attach without a preceding space but do NOT capitalize the next word.
    fn inline_punct(outline: &Outline) -> Option<Punctuation> {
        let punct = match outline.to_string().as_str() {
            "KW-BG" => ",",
            "STPH-FPLT" => ";",
            "KHR-PB" => ":",
            _ => return None,
        };
        Some(Punctuation {
            punct: punct.to_string(),
            outline: outline.clone(),
            caps_next: false,
        })
    }

    /// Convert a dictionary result to punctuation if it's a punctuation marker like "{.}".
    fn dict_to_punct(word: &str, outline: &Outline) -> Option<Punctuation> {
        let punct = match word {
            "{.}" => Some((".".to_string(), true)),
            "{!}" => Some(("!".to_string(), true)),
            "{?}" => Some(("?".to_string(), true)),
            "{,}" => Some((",".to_string(), false)),
            "{;}" => Some((";".to_string(), false)),
            "{:}" => Some((":".to_string(), false)),
            _ => return None,
        };
        punct.map(|(p, caps_after)| Punctuation {
            punct: p,
            outline: outline.clone(),
            caps_next: caps_after,
        })
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
        let result = self.preedit_string();
        self.buffer.clear();
        result
    }

    /// Clear the entire buffer (committed words, punctuation, and pending strokes).
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Push a stroke into the buffer. Returns what the engine should do.
    pub fn push_stroke(&mut self, stroke: Stroke) -> BufferAction {
        if stroke == Stroke::star() {
            return self.undo();
        }

        if Self::is_hf_only(stroke) {
            self.buffer.push_back(Element::CapsNext);
            return BufferAction::UpdatePreedit;
        } else if Self::is_hstarf_only(stroke) {
            // H*F: insert CapsNext before the pending strokes or the most recent committed word.
            return self.insert_caps_previous();
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

        // Add the new stroke to the buffer
        self.buffer.push_back(Element::Stroke(stroke));

        // Re-process from the start or from just before the rightmost committed word
        self.reprocess();

        // Return action based on whether there are pending strokes left
        if matches!(self.buffer.back(), Some(Element::Stroke(_))) {
            BufferAction::UpdatePreedit
        } else {
            BufferAction::CommitAndPreedit
        }
    }

    /// Re-process all elements in the buffer to find longest matches.
    /// Starts from just before the rightmost committed word (or from start if empty).
    fn reprocess(&mut self) {
        // Find the position of the rightmost committed word or punctuation
        let rightmost_pos = 0;
        /*
            self.buffer.iter().rposition(|el| {
                matches!(el, Element::CommittedWord(_) | Element::Punctuation(_))
            }).unwrap_or(0);
        */

        // Compute the capitalization state AT the reprocessing point
        // This is false unless there's a punctuation before this point that sets it
        let mut current_cap_state = false;
        for i in (0..rightmost_pos).rev() {
            if let Element::Punctuation(p) = &self.buffer[i] {
                current_cap_state = p.caps_next;
                break;
            }
        }

        let elements_to_reprocess: Vec<Element> = self.buffer.split_off(rightmost_pos).into();

        // Stream through the extracted elements. Non-CapsNext elements are deconstructed
        // into raw strokes and accumulated. When a CapsNext is encountered, the pending
        // strokes are matched and committed first, then the CapsNext marker is placed
        // directly into the buffer at the right position.
        let mut pending: Vec<Stroke> = Vec::new();
        let mut should_capitalize = current_cap_state;
        for el in elements_to_reprocess {
            match el {
                Element::Stroke(s) => pending.push(s),
                Element::CommittedWord(cw) => pending.extend_from_slice(cw.outline.strokes()),
                Element::Punctuation(p) => pending.extend_from_slice(p.outline.strokes()),
                Element::CapsNext => {
                    should_capitalize = self.greedy_match_and_push(&pending, should_capitalize);
                    pending.clear();
                    self.buffer.push_back(Element::CapsNext);
                }
            }
        }
        self.greedy_match_and_push(&pending, should_capitalize);
    }

    /// Parse a steno dictionary word entry, stripping brace-join notation.
    /// Words wrapped in `{...}` have their braces removed; a leading `^` sets
    /// `join_left` and a trailing `^` sets `join_right`. Plain words are returned
    /// unchanged with both flags `false`.
    fn parse_word_entry(word: &str) -> (String, bool, bool) {
        if word.starts_with('{') && word.ends_with('}') {
            let join_left = word.starts_with("{^");
            let join_right = word.ends_with("^}");
            let inner = &word[1..word.len() - 1];
            let inner = if join_left { &inner[1..] } else { inner };
            let inner = if join_right { &inner[..inner.len() - 1] } else { inner };
            (inner.to_string(), join_left, join_right)
        } else {
            (word.to_string(), false, false)
        }
    }

    /// Greedily match `strokes` against the dictionary and push the resulting elements
    /// onto `self.buffer`. Returns the capitalization state after the final element.
    fn greedy_match_and_push(&mut self, strokes: &[Stroke], mut should_capitalize: bool) -> bool {
        let mut i = 0;
        while i < strokes.len() {
            let mut best_match_len = 0;
            let mut best_match_element = None;

            for j in (i + 1)..=strokes.len() {
                let outline = Outline::from(&strokes[i..j]);

                if let Some(punct) = Self::sentence_end_punct(&outline) {
                    best_match_len = j - i;
                    best_match_element = Some(Element::Punctuation(punct));
                } else if let Some(punct) = Self::inline_punct(&outline) {
                    best_match_len = j - i;
                    best_match_element = Some(Element::Punctuation(punct));
                } else if let Some(word) = self.dictionary.lookup(outline.clone()) {
                    best_match_len = j - i;
                    if let Some(punct) = Self::dict_to_punct(&word, &outline) {
                        best_match_element = Some(Element::Punctuation(punct));
                    } else {
                        let (cleaned_word, join_left, join_right) =
                            Self::parse_word_entry(&word);
                        best_match_element = Some(Element::CommittedWord(CommittedWord {
                            word: cleaned_word,
                            outline: outline.clone(),
                            join_left,
                            join_right,
                        }));
                    }
                }
            }

            if let Some(el) = best_match_element {
                should_capitalize = if let Element::Punctuation(ref p) = el {
                    p.caps_next
                } else {
                    false
                };
                self.buffer.push_back(el);
                i += best_match_len;
            } else {
                for s in &strokes[i..] {
                    self.buffer.push_back(Element::Stroke(*s));
                }
                break;
            }
        }
        should_capitalize
    }


    /// Undo the last element or decompose the last committed word/punctuation.
    fn undo(&mut self) -> BufferAction {
        if let Some(el) = self.buffer.pop_back() {
            match el {
                Element::Stroke(_) | Element::CapsNext => {
                    // Just removed a stroke, done
                }
                Element::CommittedWord(cw) => {
                    // Decompose: restore all strokes of the word minus the final one
                    let outline_strokes = cw.outline.strokes();
                    if outline_strokes.len() > 1 {
                        for s in &outline_strokes[..outline_strokes.len() - 1] {
                            self.buffer.push_back(Element::Stroke(*s));
                        }
                    }
                }
                Element::Punctuation(p) => {
                    // Decompose: restore all strokes of the punctuation minus the final one
                    let outline_strokes = p.outline.strokes();
                    if outline_strokes.len() > 1 {
                        for s in &outline_strokes[..outline_strokes.len() - 1] {
                            self.buffer.push_back(Element::Stroke(*s));
                        }
                    }
                }
            }
            BufferAction::UpdatePreedit
        } else {
            BufferAction::SendBackspace
        }
    }

    /// Build the preedit string for display.
    /// Format: `word1 word2 STROKE1/STROKE2`
    /// Punctuation is attached without preceding space based on its rules.
    pub fn preedit_string(&self) -> String {
        log::warn!("BUFFER: {:?}", &self.buffer);
        let mut result = String::new();

        let mut caps_next = false;

        for (el, next_el) in self.buffer.iter().zip(self.buffer.iter().skip(1).map(|e| Some(e)).chain(std::iter::once(None))) {
            match el {
                Element::Stroke(s) => {
                    caps_next = false;
                    result.push_str(&s.extended());
                    if matches!(next_el, Some(Element::Stroke(_))) {
                        result.push('/');
                    }
                    if let Some(Element::CommittedWord(w)) = next_el && !w.join_left {
                        result.push(' ');
                    } else if let Some(Element::CapsNext) = next_el {
                        result.push(' ');
                    }
                }
                Element::CapsNext => {
                    if matches!(next_el, None) {
                        result.push_str("CAP");
                    }
                    caps_next = true;
                }
                Element::CommittedWord(cw) => {
                    let word = if caps_next {
                        &Self::capitalize(&cw.word)
                    } else {
                        &cw.word
                    };
                    result.push_str(&word);
                    if matches!(next_el,
                          Some(Element::CommittedWord(_))
                        | Some(Element::CapsNext)
                        | Some(Element::Stroke(_))
                    ) {
                        if !cw.join_right && let Some(Element::CommittedWord(ncw)) = next_el && !ncw.join_left {
                            result.push(' ');
                        }
                    }
                    caps_next = false;
                }
                Element::Punctuation(p) => {
                    result.push_str(&p.punct);

                    if matches!(next_el,
                          Some(Element::CommittedWord(_))
                        | Some(Element::CapsNext)
                        | Some(Element::Stroke(_))
                    ) {
                        result.push(' ');
                    }

                    caps_next = p.caps_next;
                }
            }
        }

        result
    }

    /// Number of pending strokes.
    pub fn pending_len(&self) -> usize {
        self.buffer.iter().filter(|el| matches!(el, Element::Stroke(_))).count()
    }

    /// The byte offset where pending strokes begin in the preedit string.
    /// Used for setting underline attributes.
    pub fn committed_preedit_len(&self) -> usize {
        let mut len = 0;
        let mut needs_space = false;

        for el in &self.buffer {
            match el {
                Element::Stroke(_) | Element::CapsNext => break,
                Element::CommittedWord(cw) => {
                    if needs_space {
                        len += 1; // space separator
                    }
                    len += cw.word.len();
                    needs_space = true;
                }
                Element::Punctuation(p) => {
                    len += p.punct.len();
                }
            }
        }
        len
    }

    /// Reverse lookup: find the outline for a given word.
    /// Returns None if the word is not in the dictionary.
    pub fn reverse_lookup_word(&self, word: &str) -> Option<Outline> {
        self.dictionary.reverse_lookup(word)
    }
}
