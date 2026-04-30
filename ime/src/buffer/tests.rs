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
    let state1 = SearchState::Inactive;
    assert!(matches!(state1, SearchState::Inactive));

    let state2 = SearchState::Active("hello".to_string());
    assert!(matches!(state2, SearchState::Active(ref s) if s == "hello"));

    let state3 = SearchState::ShowingResult("world".to_string());
    assert!(matches!(state3, SearchState::ShowingResult(ref s) if s == "world"));
}

#[test]
fn test_period_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("TP-PL", "{.}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("TP-PL").unwrap());

    // preedit should be "cat." with no space before the period
    assert_eq!(buf.preedit_string(), "cat.");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_exclamation_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("SKHRAPL", "{!}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("SKHRAPL").unwrap());

    assert_eq!(buf.preedit_string(), "cat!");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_question_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("KW-PL", "{?}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("KW-PL").unwrap());

    assert_eq!(buf.preedit_string(), "cat?");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_punct_capitalizes_next_word() {
    let dict = test_dictionary(&[
        ("KAT", "cat"),
        ("TP-PL", "{.}"),
        ("TKOG", "dog"),
    ]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("TP-PL").unwrap());
    buf.push_stroke(Stroke::try_from_string("TKOG").unwrap());

    // "dog" should be capitalized because it follows a period
    assert_eq!(buf.preedit_string(), "cat. Dog");
    assert_eq!(buf.committed_len(), 3);
}

#[test]
fn test_punct_flush_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("TP-PL", "{.}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("TP-PL").unwrap());

    let rr = Stroke::new(&[Key::LeftR, Key::RightR]);
    let action = buf.push_stroke(rr);
    match action {
        BufferAction::FlushAll { ref flushed } => {
            assert_eq!(flushed, "cat.");
        }
        _ => panic!("Expected FlushAll, got {:?}", action),
    }
}

#[test]
fn test_punct_committed_preedit_len() {
    let dict = test_dictionary(&[("KAT", "cat"), ("TP-PL", "{.}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("TP-PL").unwrap());

    // "cat" = 3 bytes, "." attaches with no space = 4 bytes total
    assert_eq!(buf.committed_preedit_len(), 4);
}

#[test]
fn test_comma_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("KW-BG", "{,}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("KW-BG").unwrap());

    assert_eq!(buf.preedit_string(), "cat,");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_semicolon_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("STPH-FPLT", "{;}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("STPH-FPLT").unwrap());

    assert_eq!(buf.preedit_string(), "cat;");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_colon_no_space_before() {
    let dict = test_dictionary(&[("KAT", "cat"), ("KHR-PB", "{:}")]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("KHR-PB").unwrap());

    assert_eq!(buf.preedit_string(), "cat:");
    assert_eq!(buf.committed_len(), 2);
}

#[test]
fn test_inline_punct_does_not_capitalize_next() {
    let dict = test_dictionary(&[
        ("KAT", "cat"),
        ("KW-BG", "{,}"),
        ("TKOG", "dog"),
    ]);
    let mut buf = StrokeBuffer::new(dict);

    buf.push_stroke(Stroke::try_from_string("KAT").unwrap());
    buf.push_stroke(Stroke::try_from_string("KW-BG").unwrap());
    buf.push_stroke(Stroke::try_from_string("TKOG").unwrap());

    // "dog" should NOT be capitalized after a comma
    assert_eq!(buf.preedit_string(), "cat, dog");
    assert_eq!(buf.committed_len(), 3);
}

