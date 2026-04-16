use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::{cursor, execute, terminal};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use seagull::device::Device;
use seagull::{Machine, Stroke};

struct State {
    machine: Machine,
}

impl State {
    fn new() -> Self {
        // CARGO_MANIFEST_DIR is the seagull/ package dir; data/ lives one level up.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../data/seagull.json");
        State {
            machine: Machine::from_file(path),
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
    let mut state = State::new();
    let sentences = load_sentences();
    if sentences.is_empty() {
        eprintln!("No sentences found");
        return;
    }

    //let sentence = pick_sentence(&sentences).to_string();
    let sentence = "I love you";
    let _words = tokenize(&sentence); // available for future word-matching logic

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

    // Clear screen and show the practice sentence
    terminal::enable_raw_mode().expect("Failed to enable raw mode");
    execute!(
        stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )
    .expect("Failed to clear screen");
    print!("Practice: {}\r\n\r\nStrokes:\r\n", sentence);
    stdout().flush().unwrap();

    // Main event loop
    loop {
        match rx.recv() {
            Ok(GameEvent::Stroke(stroke)) => {
                let command = state.machine.apply(stroke);
                print!("{:?}\r\n", command);
                stdout().flush().unwrap();
            }
            Ok(GameEvent::Quit) | Err(_) => break,
        }
    }

    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    execute!(stdout(), cursor::MoveToNextLine(1)).ok();
}
