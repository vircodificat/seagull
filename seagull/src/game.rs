use std::collections::HashMap;
use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::{cursor, execute, terminal};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Stylize;
use seagull::device::{Device, Keycode};
use seagull::{Dictionary, JsonDictionary, Outline, Stroke};

const SLOW_THRESHOLD: Duration = Duration::from_secs(2);

struct Stats {
    total_correct_words: usize,
    total_time: Duration,
    mistakes: HashMap<String, usize>, // target word → wrong-attempt count
    slow_words: Vec<String>,          // target words where typing took > SLOW_THRESHOLD
}

impl Stats {
    fn new() -> Self {
        Stats {
            total_correct_words: 0,
            total_time: Duration::ZERO,
            mistakes: HashMap::new(),
            slow_words: vec![],
        }
    }

    fn wpm(&self) -> f64 {
        let minutes = self.total_time.as_secs_f64() / 60.0;
        if minutes == 0.0 { 0.0 } else { self.total_correct_words as f64 / minutes }
    }
}

struct State {
    debug: bool,
    dictionary: JsonDictionary,
    sentences: Vec<String>,
    sentence: Vec<String>,  // target words
    words: Vec<String>,     // committed words so far
    word_outlines: Vec<Outline>,
    strokes: Vec<Stroke>,   // strokes building the current (uncommitted) word
    hint: Option<String>,
    is_running: bool,
    sentence_start: Option<Instant>, // when the first stroke of this sentence arrived
    word_start: Option<Instant>,     // when the current word's first stroke arrived
    stats: Stats,
}

impl State {
    fn new(sentences: Vec<String>) -> Self {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/seagull.json");
        let dictionary = JsonDictionary::load_from_file(path).unwrap();
        let mut state = State {
            debug: false,
            dictionary,
            sentences,
            sentence: vec![],
            words: vec![],
            word_outlines: vec![],
            strokes: vec![],
            hint: None,
            is_running: true,
            sentence_start: None,
            word_start: None,
            stats: Stats::new(),
        };
        state.load_new_sentence();
        state
    }

    fn load_new_sentence(&mut self) {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as usize;
        let sentence = &self.sentences[seed % self.sentences.len()];
        self.sentence = tokenize(&sentence);
        self.words.clear();
        self.word_outlines.clear();
        self.strokes.clear();
        self.hint = None;
    }

    fn current_target_word(&self) -> Option<String> {
        // The current word. That is, the word which comes NEXT in the sentence after all of
        // the words the user has correctly entered (counting the green words from the start of the
        // sentence, but stopping at the first non-green word).
        let correct_count = self.words.iter()
            .zip(self.sentence.iter())
            .take_while(|(entered, target)| entered == target)
            .count();
        self.sentence.get(correct_count).cloned()
    }

    /// Accumulate stats for the just-completed sentence, then load the next one.
    fn complete_sentence(&mut self) {
        if let Some(start) = self.sentence_start.take() {
            self.stats.total_time += start.elapsed();
            self.stats.total_correct_words += self.sentence.len();
        }
        self.word_start = None;
        self.load_new_sentence();
    }

    /// Running WPM including the current in-progress sentence.
    fn current_wpm(&self) -> f64 {
        let current_correct = self.words.iter()
            .zip(&self.sentence)
            .filter(|(a, b)| a == b)
            .count();
        let current_time = self.sentence_start.map(|s| s.elapsed()).unwrap_or(Duration::ZERO);
        let total_words = self.stats.total_correct_words + current_correct;
        let total_time  = self.stats.total_time + current_time;
        let minutes = total_time.as_secs_f64() / 60.0;
        if minutes == 0.0 { 0.0 } else { total_words as f64 / minutes }
    }

    fn print_stats(&self) {
        println!("\n--- Session Statistics ---");
        println!("WPM: {:.1}", self.stats.wpm());

        if !self.stats.mistakes.is_empty() {
            println!("\nWords with mistakes:");
            let mut m: Vec<_> = self.stats.mistakes.iter().collect();
            m.sort_by(|a, b| b.1.cmp(a.1));
            for (word, count) in m {
                println!("  \"{}\": {} time(s)", word, count);
            }
        }

        if !self.stats.slow_words.is_empty() {
            println!("\nSlow words (> {}s):", SLOW_THRESHOLD.as_secs());
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for w in &self.stats.slow_words {
                *counts.entry(w.as_str()).or_insert(0) += 1;
            }
            let mut sorted: Vec<_> = counts.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (word, count) in sorted {
                println!("  \"{}\": {} time(s)", word, count);
            }
        }
    }

}

enum GameEvent {
    Keycode(Keycode),
    Tick,
    Quit,
}

/// Split a sentence into lowercase words; punctuation becomes a separate token.
fn tokenize(sentence: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut word = String::new();
    for ch in sentence.chars() {
        if ch.is_alphabetic() || ch == '\'' {
            word.push(ch.to_ascii_lowercase());
        } else {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            if ch != ' ' {
                tokens.push(ch.to_string());
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

fn load_sentences() -> Vec<String> {
    // Embedded at compile time; path is relative to this source file.
    include_str!("../../sentences.txt")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect()
}

pub fn run(mut device: Box<dyn Device>) {
    let sentences = load_sentences();
    if sentences.is_empty() {
        eprintln!("No sentences found");
        return;
    }

    let mut state = State::new(sentences);

    let (tx, rx) = mpsc::channel::<GameEvent>();

    // Thread: read strokes from the steno machine
    let tx_serial = tx.clone();
    thread::spawn(move || {
        loop {
            let keycode = device.read_stroke();
            if tx_serial.send(GameEvent::Keycode(keycode)).is_err() {
                break;
            }
        }
    });

    // Thread: periodic tick for live WPM updates
    let tx_tick = tx.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(500));
        if tx_tick.send(GameEvent::Tick).is_err() {
            break;
        }
    });

    // Thread: watch for ESC on the keyboard
    let tx_kb = tx;
    thread::spawn(move || loop {
        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            let quit = code == KeyCode::Esc
                || (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL));
            if quit {
                let _ = tx_kb.send(GameEvent::Quit);
                break;
            }
        }
    });

    terminal::enable_raw_mode().expect("Failed to enable raw mode");
    state.render();

    // Main event loop
    while state.is_running {
        match rx.recv() {
            Ok(GameEvent::Keycode(keycode)) => {
                state.apply(keycode);
                state.render();
            }
            Ok(GameEvent::Tick) => state.render(),
            Ok(GameEvent::Quit) | Err(_) => break,
        }
    }

    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    execute!(stdout(), cursor::MoveToNextLine(1)).ok();
    state.print_stats();
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

impl State {
    fn apply(&mut self, keycode: Keycode) {
        let stroke = keycode.stroke();

        if keycode.is_control() {
            if stroke.extended() == "KWIT" {
                self.is_running = false;
            } else if stroke.extended() == "H" {
                if let Some(word) = self.current_target_word() {
                    if let Some(outline) = self.dictionary.reverse_lookup(&word) {
                        self.hint = Some(outline.extended());
                    } else {
                        self.hint = Some(format!("{word:?} is not in dictionary!"));
                    }
                }
            } else if stroke.extended() == "S" {
                if let Some(word) = self.current_target_word() {
                    self.words.push(word);
                    let outline = Outline::from(self.strokes.as_slice());
                    self.word_outlines.push(outline);
                    self.strokes.clear();

                    let correct = self.words == self.sentence;
                    if correct {
                        self.load_new_sentence();
                    }
                }
            }
            return;
        }

        if stroke == Stroke::star() {
            if self.strokes.is_empty() {
                self.words.pop();
                self.word_outlines.pop();
                // Restart word timer: user is now re-typing the undone word.
                self.word_start = Some(Instant::now());
            } else {
                self.strokes.clear();
            }
            return;
        }

        // Start sentence / word timers on the first real typing stroke.
        if self.sentence_start.is_none() {
            self.sentence_start = Some(Instant::now());
            self.word_start     = Some(Instant::now());
        }

        if self.strokes.is_empty() {
            if let Some(outline) = self.word_outlines.last() {
                let new_outline = outline.join(stroke);
                if !outline.is_empty() && let Some(word) = self.dictionary.lookup(new_outline.clone()) {
                    let committed = word.trim().to_lowercase();
                    let position  = self.words.len() - 1; // replacing last word
                    let target    = self.sentence.get(position).cloned().unwrap_or_default();
                    let old_word  = self.words[position].clone();

                    if old_word != target && committed == target {
                        // Extension corrected a previously wrong word: undo that mistake.
                        if let Some(count) = self.stats.mistakes.get_mut(&target) {
                            *count = count.saturating_sub(1);
                            if *count == 0 { self.stats.mistakes.remove(&target); }
                        }
                    } else if old_word == target && committed != target {
                        // Was correct, extension made it wrong: new mistake.
                        *self.stats.mistakes.entry(target).or_insert(0) += 1;
                    }
                    // old wrong + new still wrong: original mistake stands, no extra.
                    // old correct + new still correct: nothing to do.

                    self.word_start = Some(Instant::now());

                    self.words.pop();
                    self.word_outlines.pop();
                    self.words.push(committed);
                    self.word_outlines.push(new_outline);

                    let correct = self.words == self.sentence;
                    if correct { self.complete_sentence(); }

                    return;
                }
            }
        }

        self.strokes.push(stroke);
        let outline = Outline::from(self.strokes.as_slice());
        if let Some(word) = self.dictionary.lookup(outline.clone()) {
            let committed = word.trim().to_lowercase();
            let position  = self.words.len(); // before push
            let target    = self.sentence.get(position).cloned().unwrap_or_default();

            if committed != target {
                *self.stats.mistakes.entry(target.clone()).or_insert(0) += 1;
            }
            if let Some(ws) = self.word_start.replace(Instant::now()) {
                if ws.elapsed() > SLOW_THRESHOLD {
                    self.stats.slow_words.push(target);
                }
            }

            self.words.push(committed);
            self.word_outlines.push(outline);
            self.strokes.clear();
        }

        let correct = self.words == self.sentence;
        if correct { self.complete_sentence(); }
    }

    fn render(&self) {
        let mut out = stdout();
        execute!(out, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0)).ok();

        print!("WPM: {:.1}\r\n\r\n", self.current_wpm());

        if self.debug {
            print!("sentence:      {:?}\r\n", self.sentence);
            print!("words:         {:?}\r\n", self.words);
            print!("word_outlines: {:?}\r\n", self.word_outlines);
            print!("strokes:       {:?}\r\n", self.strokes);
            if let Some(hint) = &self.hint {
                print!("hint:          {:?}\r\n", hint);
            } else {
                print!("\r\n");
            }
            print!("\r\n");
        }

        // Target sentence (first word capitalised for display)
        let sentence_display: String = self.sentence.iter().enumerate()
            .map(|(i, w)| if i == 0 { capitalize(w) } else { w.clone() })
            .collect::<Vec<_>>()
            .join(" ");
        print!("{}\r\n\r\n", sentence_display);

        // Plain text: entered words, first letter of first word capitalised
        let plain: String = self.words.iter().enumerate()
            .map(|(i, w)| if i == 0 { capitalize(w) } else { w.clone() })
            .collect::<Vec<_>>()
            .join(" ");
        print!("{}\r\n\r\n", plain);

        // Colour-coded words: green = correct, red = incorrect
        for (i, word) in self.words.iter().enumerate() {
            if i > 0 { print!(" "); }
            let display = if i == 0 { capitalize(word) } else { word.clone() };
            if self.sentence.get(i).map(|s| s == word).unwrap_or(false) {
                print!("{}", display.on_green());
            } else {
                print!("{}", display.on_red());
            }
        }

        // Current partial strokes highlighted in yellow
        if !self.strokes.is_empty() {
            if !self.words.is_empty() { print!(" "); }
            let partial = self.strokes.iter()
                .map(|s| s.extended())
                .collect::<Vec<_>>()
                .join("/");
            print!("{}", partial.on_yellow());
        }

        print!("\r\n");
        out.flush().ok();
    }
}
