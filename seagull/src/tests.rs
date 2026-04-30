use seagull::{Outline, Stroke};

// Regression tests - ensure existing behavior is not broken
#[test]
fn test_initial_n_without_dash() {
    // "N" without dash should parse as INITIAL N (TPH = LeftT+LeftP+LeftH)
    let stroke = Stroke::try_from_extended("N".into()).unwrap();
    // Initial "N" at index 44 in INITIALS array
    let outline = Outline::from(stroke);
    assert!(!outline.to_string().is_empty(), "Should parse to a valid outline");
}

#[test]
fn test_extended_with_initials() {
    // Extended notation with initials should work (e.g., "BRASh")
    let outline = Outline::try_from_extended("BRASh".into());
    assert!(outline.is_some(), "Should parse extended notation with initials");
}

#[test]
fn test_standard_steno_notation() {
    // Standard steno notation should still work
    let outline1 = Outline::try_from_extended("S".into());
    let outline2 = Outline::try_from_extended("P-FL".into());
    assert!(outline1.is_some(), "Should parse simple left-side stroke");
    assert!(outline2.is_some(), "Should parse compound steno notation");
}

#[test]
fn test_extended_without_dash() {
    // Extended notation without dash should work normally
    let outline = Outline::try_from_extended("D".into());
    // "D" is TPH cluster in extended notation
    assert!(outline.is_some(), "Should parse extended D");
}

// Main test - the fix target
#[test]
fn test_n() {
    let outline1 = Outline::try_from_extended("-N".into()).unwrap();
    let outline2 = Outline::try_from_extended("-PB".into()).unwrap();
    assert_eq!(outline1, outline2);
}

#[test]
fn test_as() {
    assert_eq!(Outline::try_from_string("AS").unwrap().extended(), "AS".to_string());
}

#[test]
fn test_corner_cases() {
    // Test extended notation with initial and final separated by dash
    assert_eq!(Outline::try_from_extended("B-N".into()).unwrap().to_string(), "PW-PB".to_string());
    assert_eq!(Outline::try_from_extended("B-N".into()).unwrap().extended(), "B-N".to_string());

    // Test extended notation with initial and final (no dash separator)
    assert_eq!(Outline::try_from_extended("SL".into()).unwrap().to_string(), "SHR".to_string());
    assert_eq!(Outline::try_from_extended("SL".into()).unwrap().extended(), "SL".to_string());

    assert_eq!(Outline::try_from_extended("B-G".into()).unwrap().to_string(), "PW-G".to_string());
    assert_eq!(Outline::try_from_extended("B-G".into()).unwrap().extended(), "B-G".to_string());
}
