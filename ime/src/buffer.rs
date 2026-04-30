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
}

/// Punctuation mark in the buffer.
#[derive(Debug, Clone)]
pub struct Punctuation {
    pub punct: String,
    pub outline: Outline,
    pub space_after: bool,
    pub caps_after: bool,
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
            space_after: false,
            caps_after: true,
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
            space_after: true,
            caps_after: false,
        })
    }

    /// Convert a dictionary result to punctuation if it's a punctuation marker like "{.}".
    fn dict_to_punct(word: &str, outline: &Outline) -> Option<Punctuation> {
        let punct = match word {
            "{.}" => Some((".".to_string(), false, true)),
            "{!}" => Some(("!".to_string(), false, true)),
            "{?}" => Some(("?".to_string(), false, true)),
            "{,}" => Some((",".to_string(), true, false)),
            "{;}" => Some((";".to_string(), true, false)),
            "{:}" => Some((":".to_string(), true, false)),
            _ => return None,
        };
        punct.map(|(p, space_after, caps_after)| Punctuation {
            punct: p,
            outline: outline.clone(),
            space_after,
            caps_after,
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
        let rightmost_pos = self.buffer.iter().rposition(|el| {
            matches!(el, Element::CommittedWord(_) | Element::Punctuation(_))
        }).unwrap_or(0);

        // Compute the capitalization state AT the reprocessing point
        // This is false unless there's a punctuation before this point that sets it
        let mut current_cap_state = false;
        for i in (0..rightmost_pos).rev() {
            if let Element::Punctuation(p) = &self.buffer[i] {
                current_cap_state = p.caps_after;
                break;
            }
        }

        // Extract elements from reprocess_from onwards
        let elements_to_reprocess: Vec<Element> = self.buffer.split_off(rightmost_pos).into();

        // Collect all strokes from elements to reprocess and track capitalized words
        let mut all_strokes = Vec::new();
        let mut capitalized_at: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
        let mut stroke_index = 0;
        for el in &elements_to_reprocess {
            match el {
                Element::Stroke(s) => {
                    all_strokes.push(*s);
                    stroke_index += 1;
                }
                Element::CapsNext => {
                    capitalized_at.insert(stroke_index, true);
                }
                Element::CommittedWord(cw) => {
                    let _is_cap = cw.word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
                    all_strokes.extend_from_slice(cw.outline.strokes());
                    stroke_index = all_strokes.len();
                }
                Element::Punctuation(p) => {
                    all_strokes.extend_from_slice(p.outline.strokes());
                    stroke_index = all_strokes.len();
                }
            }
        }

        // Reprocess strokes to find longest matches
        let mut i = 0;
        let mut should_capitalize = current_cap_state;
        while i < all_strokes.len() {
            let mut best_match_len = 0;
            let mut best_match_element = None;

            // Try all possible lengths from current position
            for j in (i + 1)..=all_strokes.len() {
                let outline = Outline::from(&all_strokes[i..j]);

                // Check sentence-ending punctuation first
                if let Some(punct) = Self::sentence_end_punct(&outline) {
                    best_match_len = j - i;
                    best_match_element = Some(Element::Punctuation(punct));
                }
                // Then check inline punctuation
                else if let Some(punct) = Self::inline_punct(&outline) {
                    best_match_len = j - i;
                    best_match_element = Some(Element::Punctuation(punct));
                }
                // Then check dictionary for words
                else if let Some(word) = self.dictionary.lookup(outline.clone()) {
                    best_match_len = j - i;
                    // Check if the dictionary result is a punctuation marker
                    if let Some(punct) = Self::dict_to_punct(&word, &outline) {
                        best_match_element = Some(Element::Punctuation(punct));
                    } else {
                        let mut w = word.to_string();
                        // Capitalize if: should_capitalize is set OR any consumed word was capitalized
                        let was_consumed_capitalized = capitalized_at.get(&i).copied().unwrap_or(false);
                        if should_capitalize || was_consumed_capitalized {
                            w = Self::capitalize(&w);
                        }
                        best_match_element = Some(Element::CommittedWord(CommittedWord {
                            word: w,
                            outline: outline.clone(),
                        }));
                    }
                }
            }

            if let Some(el) = best_match_element {
                // Update capitalization state if this is punctuation
                if let Element::Punctuation(ref p) = el {
                    should_capitalize = p.caps_after;
                } else {
                    should_capitalize = false;
                }
                self.buffer.push_back(el);
                i += best_match_len;
            } else {
                // No match: add as stroke and stop
                for j in i..all_strokes.len() {
                    self.buffer.push_back(Element::Stroke(all_strokes[j]));
                }
                break;
            }
        }
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
        let mut result = String::new();

        let mut caps_next = false;
        let len = self.buffer.len();
        log::error!("$$$$$$$$$$$$$$$$$$$$$$$  {len:?}");

        let mut insert_wedge_space = false;
        for (i, el) in self.buffer.iter().enumerate() {
            let _is_last = i + 1 == self.buffer.len();
            log::error!("======  {el:?}");

            match el {
                Element::Stroke(s) => {
                    caps_next = false;
                    result.push_str(&s.extended());
                }
                Element::CapsNext => {
                    info!("CapsNext");
                    log::error!("CapsNext");
                    caps_next = true;
                }
                Element::CommittedWord(cw) => {
                    if insert_wedge_space {
                        log::error!("!!!!!!!!     INSERT WEDGE SPACE");
                        result.push(' ');
                    }

                    let word = if caps_next {
                        log::error!("!!!!!!!!     Making next word caps");
                        &Self::capitalize(&cw.word)
                    } else {
                        log::error!("!!!!!!!!     Just regular next word");
                        &cw.word
                    };
                    result.push_str(&word);
                    caps_next = false;
                    insert_wedge_space = true;
                }
                Element::Punctuation(p) => {
                    result.push_str(&p.punct);
                    // Add space after if needed
                    if p.space_after && !result.is_empty() {
                        // Mark that next element needs space
                        insert_wedge_space = true;
                    } else {
                        insert_wedge_space = false;
                    }

                    caps_next = false;
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
                    needs_space = p.space_after;
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
