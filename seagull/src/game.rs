use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::{cursor, execute, terminal};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Stylize;
use seagull::device::Device;
use seagull::{Dictionary, JsonDictionary, Outline, Stroke};

struct State {
    debug: bool,
    dictionary: JsonDictionary,
    sentence: Vec<String>,  // target words
    words: Vec<String>,     // committed words so far
    word_outlines: Vec<Outline>,
    strokes: Vec<Stroke>,   // strokes building the current (uncommitted) word
}

impl State {
    fn new(sentence: Vec<String>) -> Self {
        // CARGO_MANIFEST_DIR is the seagull/ package dir; data/ lives one level up.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/seagull.json");
        let dictionary = JsonDictionary::load_from_file(path).unwrap();
        State {
            debug: true,
            dictionary,
            sentence,
            words: vec![],
            word_outlines: vec![],
            strokes: vec![],
        }
    }
}

enum GameEvent {
    Stroke(Stroke),
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

fn pick_sentence(sentences: &[String]) -> &str {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    &sentences[seed % sentences.len()]
}

pub fn run(mut device: Box<dyn Device>) {
    let sentences = load_sentences();
    if sentences.is_empty() {
        eprintln!("No sentences found");
        return;
    }

    // let sentence = pick_sentence(&sentences).to_string();
    let sentence = "I love you".to_string();
    let words = tokenize(&sentence);
    let mut state = State::new(words);

    let (tx, rx) = mpsc::channel::<GameEvent>();

    // Thread: read strokes from the steno machine
    let tx_serial = tx.clone();
    thread::spawn(move || {
        loop {
            let stroke = device.read_stroke();
            if tx_serial.send(GameEvent::Stroke(stroke)).is_err() {
                break;
            }
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
    loop {
        match rx.recv() {
            Ok(GameEvent::Stroke(stroke)) => {
                state.apply(stroke);
                state.render();
            }
            Ok(GameEvent::Quit) | Err(_) => break,
        }
    }

    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    execute!(stdout(), cursor::MoveToNextLine(1)).ok();
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

impl State {
    fn apply(&mut self, stroke: Stroke) {
        if stroke == Stroke::star() {
            if self.strokes.is_empty() {
                self.words.pop();
            } else {
                self.strokes.clear();
            }
            return;
        } else {
            if self.strokes.is_empty() {
                if let Some(outline) = self.word_outlines.last() {
                    let new_outline = outline.join(stroke);
                    if let Some(word) = self.dictionary.lookup(new_outline.clone()) {
                        self.words.pop();
                        self.word_outlines.pop();

                        self.words.push(word.to_string());
                        self.word_outlines.push(new_outline);
                        return;
                    }
                }
            }

            self.strokes.push(stroke);
            let outline = Outline::try_from(self.strokes.as_slice()).unwrap();
            if let Some(word) = self.dictionary.lookup(outline.clone()) {
                self.words.push(word.trim().to_lowercase());
                self.word_outlines.push(outline);
                self.strokes.clear();
            }
        }
    }

    fn render(&self) {
        let mut out = stdout();
        execute!(out, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0)).ok();

        if self.debug {
            print!("sentence:      {:?}\r\n", self.sentence);
            print!("words:         {:?}\r\n", self.words);
            print!("word_outlines: {:?}\r\n", self.word_outlines);
            print!("strokes:       {:?}\r\n", self.strokes);
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
