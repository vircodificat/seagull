#![allow(clippy::collapsible_if, clippy::new_without_default, unused_parens, clippy::needless_return, clippy::len_without_is_empty)]

pub mod device;
pub mod extended;
#[cfg(test)] mod prefix_tree_test;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;
use serialport::SerialPort;

// Keyboard order: S T K P W H R | A O * E U | F R P B L G T S D Z
// Each variant occupies its own bit; Stroke is the bitwise OR of its keys.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[repr(u32)]
pub enum Key {
    LeftS      = 1 << 0,
    LeftT      = 1 << 1,
    LeftK      = 1 << 2,
    LeftP      = 1 << 3,
    LeftW      = 1 << 4,
    LeftH      = 1 << 5,
    LeftR      = 1 << 6,

    MiddleA    = 1 << 7,
    MiddleO    = 1 << 8,
    MiddleStar = 1 << 9,
    MiddleE    = 1 << 10,
    MiddleU    = 1 << 11,

    RightF     = 1 << 12,
    RightR     = 1 << 13,
    RightP     = 1 << 14,
    RightB     = 1 << 15,
    RightL     = 1 << 16,
    RightG     = 1 << 17,
    RightT     = 1 << 18,
    RightS     = 1 << 19,
    RightD     = 1 << 20,
    RightZ     = 1 << 21,
}

const ALL_KEYS: &[Key] = &[
    Key::LeftS, Key::LeftT, Key::LeftK, Key::LeftP, Key::LeftW, Key::LeftH, Key::LeftR,
    Key::MiddleA, Key::MiddleO, Key::MiddleStar, Key::MiddleE, Key::MiddleU,
    Key::RightF, Key::RightR, Key::RightP, Key::RightB, Key::RightL,
    Key::RightG, Key::RightT, Key::RightS, Key::RightD, Key::RightZ,
];

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Stroke(u32);

pub struct PrefixTree(RefCell<(Option<String>, HashMap<Stroke, Rc<PrefixTree>>)>);

impl PrefixTree {
    pub fn new(map: HashMap<Outline, String>) -> Self {
        let tree = PrefixTree(RefCell::new((None, HashMap::new())));
        for (outline, word) in map.into_iter() {
            tree.add(outline.strokes(), word);
        }
        tree
    }

    fn add(&self, strokes: &[Stroke], word: String) {
        let PrefixTree(refcell) = self;
        if strokes.is_empty() {
            let curr_word = &mut refcell.borrow_mut().0;
            curr_word.replace(word);
        } else {
            let children = &mut refcell.borrow_mut().1;
            let head = strokes[0].clone();
            let tail = &strokes[1..];

            if children.contains_key(&head) {
                let child = children[&head].clone();
                child.add(tail, word);
            } else {
                children.insert(head.clone(), Rc::new(PrefixTree(RefCell::new((None, HashMap::new())))));
                children[&head].add(tail, word);
            }
        }
    }

    pub fn lookup(&self, outline: Outline) -> Option<String> {
        self.lookup_by_strokes(outline.strokes())
    }

    fn lookup_by_strokes(&self, strokes: &[Stroke]) -> Option<String> {
        let PrefixTree(refcell) = self;
        if strokes.is_empty() {
            let curr_word  = &refcell.borrow().0;
            curr_word.clone()
        } else {
            let children = &refcell.borrow().1;
            let head = strokes[0].clone();
            let tail = &strokes[1..];
            let child = children[&head].clone();
            child.lookup_by_strokes(tail)
        }

    }

    pub fn contains(&self, outline: Outline) -> bool {
        self.lookup_by_strokes(outline.strokes()).is_some()
    }

    pub fn following_strokes(&self, outline: Outline) -> Vec<Stroke> {
        self.following_strokes_by_strokes(outline.strokes())
    }

    fn following_strokes_by_strokes(&self, strokes: &[Stroke]) -> Vec<Stroke> {
        if strokes.is_empty() {
            let PrefixTree(refcell) = self;
            let children = &refcell.borrow().1;
            return children.keys().cloned().collect();
        }
        let PrefixTree(refcell) = self;
        let children = &refcell.borrow().1;
        let head = strokes[0].clone();
        let tail = &strokes[1..];
        if let Some(child) = children.get(&head) {
            child.following_strokes_by_strokes(tail)
        } else {
            Vec::new()
        }
    }

    pub fn prefix_strokes(&self, outline: Outline) -> Vec<Stroke> {
        self.prefix_strokes_by_strokes(outline.strokes())
    }

    fn prefix_strokes_by_strokes(&self, strokes: &[Stroke]) -> Vec<Stroke> {
        let mut prefix_strokes_rev = Vec::new();
        self.prefix_strokes_helper(strokes, &mut prefix_strokes_rev);
        let mut results = Vec::new();
        results.extend(prefix_strokes_rev.into_iter().rev());
        results
    }

    fn prefix_strokes_helper(&self, strokes: &[Stroke], prefix_strokes_rev: &mut Vec<Stroke>) {
        if let Some((stroke, rest)) = strokes.split_first() {
            let PrefixTree(refcell) = self;
            let borrowed = refcell.borrow();
            if borrowed.0.is_some() {
                prefix_strokes_rev.push(*stroke);
            }
            if let Some(child) = borrowed.1.get(stroke) {
                child.prefix_strokes_helper(rest, prefix_strokes_rev);
            }
        }
    }
}

pub fn prefix_tree_from_json_dictionary(dictionary: JsonDictionary) -> PrefixTree {
    let JsonDictionary(dict) = dictionary;
    let mut map: HashMap<Outline, String> = HashMap::new();
    for (k, v) in dict.iter() {
        if k.chars().any(|c| c.is_ascii_digit() || c == '#') { continue; }

        if let Some(outline) = Outline::try_from_string(k) {
            assert_eq!(&format!("{outline}"), k);
            map.insert(outline, v.to_string());
        } else {
            eprintln!("COULD NOT PARSE OUTLINE: {k}");
        }
    }
    PrefixTree::new(map)
}


const MAX_UNDO: usize = 1 << 15;


pub struct Machine {
    undo: Vec<Outline>,
    dictionary: Box<dyn Dictionary>,
}


impl Machine {
    pub fn new() -> Self {
        let filename: &str = "../output/main.json";
        let dictionary = Box::new(JsonDictionary::load_from_file(filename).unwrap());
        Self {
            undo: Vec::new(),
            dictionary
        }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Self {
        let dictionary = Box::new(JsonDictionary::load_from_file(path).unwrap());
        Self {
            undo: Vec::new(),
            dictionary,
        }
    }

    fn current_outline(&self, stroke: Stroke) -> Outline {
       if self.undo.is_empty() {
            Outline::from(stroke)
        } else {
           let previous_outline = self.undo[self.undo.len() - 1].clone();
           previous_outline / stroke
        }
    }

    fn limit_undo(&mut self) {
        if self.undo.len() > MAX_UNDO {
            self.undo = self.undo[self.undo.len() / 2..].to_vec();
        }
    }

    fn apply_lookup(&mut self, stroke: Stroke) -> Command {
        let outline = Outline::from(stroke);
        if let Some(word) = self.dictionary.lookup(outline) {
            Command(0, word.to_owned())
        } else {
            Command(0, String::new())
        }
    }

    fn apply_undo(&mut self) -> Command {
        Command(0, String::from("Hello"))
    }

    pub fn apply(&mut self, stroke: Stroke) -> Command {
        self.limit_undo();

        if stroke == Stroke::new(&[Key::MiddleStar]) {
            self.apply_undo()
        } else {
            self.apply_lookup(stroke)
        }
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Outline(Vec<Stroke>);

impl Outline {
    pub fn strokes(&self) -> &[Stroke] {
        let Outline(strokes) = self;
        strokes
    }

    pub fn len(&self) -> usize {
        self.strokes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.strokes().is_empty()
    }

    pub fn extended(&self) -> String {
        self.strokes()
            .into_iter()
            .map(|stroke| stroke.extended())
            .collect::<Vec<_>>()
            .join("/")
    }

    pub fn try_from_extended(s: String) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if let Some(strokes) = parts.iter()
            .map(|part| Stroke::try_from_extended(part.to_string()))
            .collect::<Option<Vec<_>>>() {
            Some(Outline::from(strokes))
        } else {
            None
        }
    }

    pub fn try_from_string(s: &str) -> Option<Outline> {
        let strokes: Vec<&str> = s.split('/').collect();
        let first = strokes.get(0)?;
        let mut outline = Outline::from(Stroke::try_from_string(first)?);

        for stroke in &strokes[1..] {
            outline = outline / Stroke::try_from_string(stroke)?;
        }

        Some(outline)
    }

    pub fn join(&self, stroke: Stroke) -> Outline {
        let mut strokes = self.strokes().to_vec();
        strokes.push(stroke);
        Outline(strokes)
    }
}

fn char_to_key(ch: char, right_side: bool) -> Option<Key> {
    let iter: Vec<_> = if right_side {
        KEY_CHARS.iter().rev().collect()
    } else {
        KEY_CHARS.iter().collect()
    };

    for (target_key, letter) in  iter {
        if ch == *letter {
            return Some(*target_key);
        }
    }
    None
}

const MIDDLE_CHARS: &[char] = &['A', 'O', '*', 'E', 'U'];

impl std::ops::Div for Outline {
    type Output = Outline;

    fn div(self, rhs: Self) -> Self::Output {
        let Outline(self_strokes) = self;
        let Outline(rhs_strokes) = &rhs;
        let result_strokes: Vec<Stroke> = self_strokes.iter().chain(rhs_strokes.iter()).cloned().collect();
        Outline(result_strokes)
    }
}

impl std::ops::Div<Stroke> for Outline {
    type Output = Outline;

    fn div(self, rhs: Stroke) -> Self::Output {
        let Outline(self_strokes) = self;
        let mut result_strokes: Vec<Stroke> = self_strokes.clone();
        result_strokes.push(rhs);
        Outline(result_strokes)
    }
}

impl From<Stroke> for Outline {
    fn from(stroke: Stroke) -> Self {
        Outline(vec![stroke])
    }
}

impl From<&[Stroke]> for Outline {
    fn from(strokes: &[Stroke]) -> Self {
        Outline(strokes.to_vec())
    }
}

impl From<Vec<Stroke>> for Outline {
    fn from(strokes: Vec<Stroke>) -> Self {
        Outline(strokes)
    }
}

impl Display for Outline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let strokes = self.strokes();
        for (i, stroke) in strokes.iter().enumerate() {
            if i > 0 {
                write!(f, "/{stroke}")?;
            } else {
                write!(f, "{stroke}")?;
            }
        }
        Ok(())
    }
}

pub trait Dictionary {
    fn lookup(&self, outline: Outline) -> Option<&str>;
    fn reverse_lookup(&self, word: &str) -> Option<Outline>;
}

pub struct JsonDictionary(HashMap<String, String>);

impl JsonDictionary {
    pub fn load_from_file<P: AsRef<std::path::Path>>(filepath: P) -> Result<Self, Box<dyn std::error::Error>> {
        let dictionary_json = std::fs::read_to_string(filepath)?;
        let parsed = json::parse(&dictionary_json)?;
        let mut dictionary: HashMap<String, String> = HashMap::new();
        for (key, value) in parsed.entries() {
            if let Some(v) = value.as_str() {
                dictionary.insert(key.to_owned(), v.to_owned());
            }
        }
        Ok(JsonDictionary(dictionary))
    }
}

impl Dictionary for JsonDictionary {
    fn lookup(&self, outline: Outline) -> Option<&str> {
        let JsonDictionary(dictionary) = self;
        let entry = dictionary.get(&outline.to_string());
        entry.map(|s| s.as_str())
    }

    fn reverse_lookup(&self, word: &str) -> Option<Outline> {
        let JsonDictionary(dictionary) = self;
        for (outline_str, entry_word) in dictionary {
            if entry_word == word {
                if let Some(outline) = Outline::try_from_string(outline_str) {
                    return Some(outline);
                }
            }
        }
        None
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Command(pub usize, pub String);

const INITIALS_MASK: u32 = (1 << 7) - 1;           // bits 0–6:  LeftS..LeftR
const MIDDLES_MASK:  u32 = ((1 << 5) - 1) << 7;    // bits 7–11: MiddleA..MiddleU
const FINALS_MASK:   u32 = ((1 << 10) - 1) << 12;  // bits 12–21: RightF..RightZ

impl Stroke {
    pub fn new(keys: &[Key]) -> Self {
        let bits = keys.iter().fold(0u32, |acc, k| acc | (*k as u32));
        Stroke(bits)
    }

    pub fn star() -> Self {
        Stroke::new(&[Key::MiddleStar])
    }

    pub fn contains(self, key: Key) -> bool {
        self.0 & (key as u32) != 0
    }

    pub fn initials(&self) -> Stroke { Stroke(self.0 & INITIALS_MASK) }
    pub fn middles(&self)  -> Stroke { Stroke(self.0 & MIDDLES_MASK) }
    pub fn finals(&self)   -> Stroke { Stroke(self.0 & FINALS_MASK) }

    pub fn keys(self) -> Vec<Key> {
        ALL_KEYS.iter().copied().filter(|&k| self.contains(k)).collect()
    }

    pub fn to_outline(self) -> Outline {
        Outline(vec![self])
    }

    pub fn try_from_string(s: &str) -> Option<Self> {
        let mut left_side = true;
        let mut keys: Vec<Key> = vec![];
        let mut last_bit: u32 = 0;
        let mut has_explicit_dash = false;
        let mut has_middle = false;

        for ch in s.chars() {
            if ch == '-' {
                left_side = false;
                has_explicit_dash = true;
                continue;
            }
            if MIDDLE_CHARS.contains(&ch) {
                left_side = false;
                has_middle = true;
            }
            let key = char_to_key(ch, !left_side)?;
            let bit = key as u32;
            if bit <= last_bit {
                return None;
            }
            last_bit = bit;
            keys.push(key);
        }

        // Check if we have finals (right-side keys) but no middles
        let has_finals = keys.iter().any(|&key| {
            matches!(key_side(key), KeySide::Right)
        });

        // If we have finals but no middles, we must have an explicit dash
        if has_finals && !has_middle && !has_explicit_dash {
            return None;
        }

        Some(Stroke::new(&keys))
    }

    pub fn try_from_extended(s: String) -> Option<Self> {
        // Standard steno strings take priority (e.g. "S" = LeftS, "-F" = RightF).
        if let Some(stroke) = Self::try_from_string(&s) {
            return Some(stroke);
        }
        // Extended phonetic lookup (e.g. "BRASh", "D" = TK cluster).
        for (i_idx, &initial) in extended::INITIALS.iter().enumerate() {
            let Some(rest) = s.strip_prefix(initial) else { continue };
            for (m_idx, &middle) in extended::MIDDLES.iter().enumerate() {
                let Some(final_str) = rest.strip_prefix(middle) else { continue };
                if let Some(f_idx) = extended::FINALS.iter().position(|&f| f == final_str) {
                    let bits = (i_idx as u32)
                        | ((m_idx as u32) << 7)
                        | ((f_idx as u32) << 12);
                    return Some(Stroke(bits));
                }
            }
        }
        None
    }

    pub fn extended(&self) -> String {
        let initial_index = self.0 & INITIALS_MASK;
        let middle_index = (self.0 & MIDDLES_MASK) >> 7;
        let finals_index = (self.0 & FINALS_MASK) >> 12;

        let initials = extended::INITIALS[initial_index as usize].to_string();
        let middles = if middle_index == 0 && finals_index != 0 {
            "-"
        } else {
            &extended::MIDDLES[middle_index as usize]
        };
        let finals = &extended::FINALS[finals_index as usize];

        initials + middles + finals
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_extended() {
        let stroke = Stroke::try_from_string("P-FL").unwrap();
        assert_eq!(stroke.extended(), "P-FL");
    }

    #[test]
    fn test_try_from_string() {
        assert!(Stroke::try_from_string("P-FL").is_some());
        assert!(Stroke::try_from_string("PFL").is_none());
    }

    #[test]
    fn test() {
        let keys = &[
            Key::LeftS,
            Key::LeftT,
            Key::LeftK,
            Key::MiddleA,
            Key::RightS,
        ];
        assert_eq!(Stroke::try_from_string("STKAS"), Some(Stroke::new(keys)));

        let keys = &[
            Key::LeftS,
            Key::LeftT,
            Key::LeftK,
            Key::RightS,
        ];
        assert_eq!(Stroke::try_from_string("STK-S"), Some(Stroke::new(keys)));
    }


    #[test]
    fn test3() {
        assert!(Outline::try_from_string("KAT").is_some());
        assert!(Outline::try_from_string("KAT/ER").is_some());
        assert!(Outline::try_from_string("BAT/TER").is_none()); // B is right-hand, can't precede A
    }

    #[test]
    fn test2() {
        let filename: &str = "../data/main.json";
        let dictionary = JsonDictionary::load_from_file(filename).unwrap();
        let prefix_tree = prefix_tree_from_json_dictionary(dictionary);
//        let outline = Outline::try_from_string("KAT").unwrap();
//        assert_eq!(prefix_tree.lookup(outline), Some("cat".to_owned()));
    }

    #[test]
    fn extended() {
        let tests = &[
            ("PWRARB", "BRASh"),
        ];
        for (stroke, ex_stroke) in tests {
            let stroke = Stroke::try_from_string(stroke).unwrap();
            assert_eq!(stroke.extended(), *ex_stroke);
        }
    }
}


impl std::fmt::Display for Stroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut has_middle = false;

        for &key in ALL_KEYS {
            if !self.contains(key) {
                continue;
            }
            let side = key_side(key);
            if side == KeySide::Middle {
                has_middle = true;
            } else if side == KeySide::Right && !has_middle {
                write!(f, "-")?;
                has_middle = true;
            }
            write!(f, "{}", key_letter(key))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
enum KeySide {
    Left,
    Middle,
    Right,
}

fn key_side(key: Key) -> KeySide {
    for (target_key, side) in KEY_SIDES {
        if key == *target_key {
            return *side;
        }
    }
    unreachable!()
}

fn key_letter(key: Key) -> char {
    for (target_key, letter) in KEY_CHARS {
        if key == *target_key {
            return *letter;
        }
    }
    unreachable!()

}

const KEY_CHARS: &[(Key, char)] = &[
    (Key::LeftS, 'S'),
    (Key::LeftT, 'T'),
    (Key::LeftK, 'K'),
    (Key::LeftP, 'P'),
    (Key::LeftW, 'W'),
    (Key::LeftH, 'H'),
    (Key::LeftR, 'R'),

    (Key::MiddleA, 'A'),
    (Key::MiddleO, 'O'),
    (Key::MiddleStar, '*'),
    (Key::MiddleE, 'E'),
    (Key::MiddleU, 'U'),

    (Key::RightF, 'F'),
    (Key::RightR, 'R'),
    (Key::RightP, 'P'),
    (Key::RightB, 'B'),
    (Key::RightL, 'L'),
    (Key::RightG, 'G'),
    (Key::RightT, 'T'),
    (Key::RightS, 'S'),
    (Key::RightD, 'D'),
    (Key::RightZ, 'Z'),
];

const KEY_SIDES: &[(Key, KeySide)] = &[
    (Key::LeftS, KeySide::Left),
    (Key::LeftT, KeySide::Left),
    (Key::LeftK, KeySide::Left),
    (Key::LeftP, KeySide::Left),
    (Key::LeftW, KeySide::Left),
    (Key::LeftH, KeySide::Left),
    (Key::LeftR, KeySide::Left),

    (Key::MiddleA, KeySide::Middle),
    (Key::MiddleO, KeySide::Middle),
    (Key::MiddleStar, KeySide::Middle),
    (Key::MiddleE, KeySide::Middle),
    (Key::MiddleU, KeySide::Middle),

    (Key::RightF, KeySide::Right),
    (Key::RightR, KeySide::Right),
    (Key::RightP, KeySide::Right),
    (Key::RightB, KeySide::Right),
    (Key::RightL, KeySide::Right),
    (Key::RightG, KeySide::Right),
    (Key::RightT, KeySide::Right),
    (Key::RightS, KeySide::Right),
    (Key::RightD, KeySide::Right),
    (Key::RightZ, KeySide::Right),
];

impl std::fmt::Debug for Stroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Debug for Outline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
