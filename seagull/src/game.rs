use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::{cursor, execute, terminal};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Stylize;
use seagull::device::{Device, Keycode};
use seagull::{Dictionary, JsonDictionary, Outline, Stroke};

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
}

impl State {
    fn new(sentences: Vec<String>) -> Self {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/seagull.json");
        let dictionary = JsonDictionary::load_from_file(path).unwrap();
        let mut state = State {
            debug: true,
            dictionary,
            sentences,
            sentence: vec![],
            words: vec![],
            word_outlines: vec![],
            strokes: vec![],
            hint: None,
            is_running: true,
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

}

enum GameEvent {
    Keycode(Keycode),
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
            } else {
                self.strokes.clear();
            }
            return;
        }

        if self.strokes.is_empty() {
            if let Some(outline) = self.word_outlines.last() {
                let new_outline = outline.join(stroke);
                if !outline.is_empty() && let Some(word) = self.dictionary.lookup(new_outline.clone()) {
                    self.words.pop();
                    self.word_outlines.pop();

                    self.words.push(word.to_string());
                    self.word_outlines.push(new_outline);

                    let correct = self.words == self.sentence;
                    if correct {
                        self.load_new_sentence();
                    }

                    return;
                }
            }
        }

        self.strokes.push(stroke);
        let outline = Outline::from(self.strokes.as_slice());
        if let Some(word) = self.dictionary.lookup(outline.clone()) {
            self.words.push(word.trim().to_lowercase());
            self.word_outlines.push(outline);
            self.strokes.clear();
        }

        let correct = self.words == self.sentence;
        if correct {
            self.load_new_sentence();
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
