use crate::{JsonDictionary, Outline, PrefixTree, outline, prefix_tree_from_json_dictionary, stroke};

fn test_tree() -> PrefixTree {
    let mut tree = PrefixTree::new();
    tree.add(outline!("KAT"), "cat".to_string());
    tree.add(outline!("KAT/ER"), "cater".to_string());
    tree.add(outline!("KAT/ER/S"), "caters".to_string());
    tree.add(outline!("SKAT"), "scat".to_string());
    tree.add(outline!("SKAT/ER"), "scatter".to_string());
    tree.add(outline!("SKAT/ER/PWRAEUPB"), "scatterbrain".to_string());
    tree
}

#[test]
fn test_contains() {
    let tree = test_tree();
    assert!(tree.contains(&outline!("KAT")));
    assert!(tree.contains(&outline!("KAT/ER")));
    assert!(tree.contains(&outline!("KAT/ER/S")));
    assert!(!tree.contains(&outline!("KAT/K")));
    assert!(!tree.contains(&outline!("S/KAT")));
}

#[test]
fn test_following_strokes() {
    let tree = test_tree();
    let following = tree.following_strokes(outline!("KAT"));
    assert!(following.contains(&stroke!("ER")));

    let following_empty = tree.following_strokes(outline!("KAT/ER/S"));
    assert!(following_empty.is_empty());

    let following_none = tree.following_strokes(outline!("KAT/-Z"));
    assert!(following_none.is_empty());
}

#[test]
#[ignore]
fn test_prefix_strokes() {
    let tree = test_tree();

    let prefixes = tree.prefix_strokes(outline!("KAT/ER"));
    assert_eq!(prefixes.len(), 1);
    assert_eq!(prefixes[0], stroke!("KAT"));

    let prefixes_multi = tree.prefix_strokes(outline!("SKAT/ER/PWRAEUPB"));
    assert_eq!(prefixes_multi.len(), 2);

    let no_prefix = tree.prefix_strokes(outline!("KAT/-Z"));
    assert!(no_prefix.is_empty());
}

#[test]
#[ignore]
fn test_lookup() {
    let tree = test_tree();
    assert_eq!(tree.lookup(outline!("KAT")), Some("cat".to_string()));
    assert_eq!(tree.lookup(outline!("KAT/ER")), Some("cater".to_string()));
    assert_eq!(tree.lookup(outline!("KAT/ER/S")), Some("caters".to_string()));
    assert_eq!(tree.lookup(outline!("-Z")), None);
}

#[test]
#[ignore]
fn test_from_main_json() {
    let dictionary = JsonDictionary::load_from_file("../data/main.json").unwrap();
    let tree = prefix_tree_from_json_dictionary(dictionary);

    assert!(tree.contains(&outline!("KAT")));
    assert!(tree.contains(&outline!("KAT/ER")));

    let following = tree.following_strokes(outline!("KAT"));
    assert!(!following.is_empty());

    let prefixes = tree.prefix_strokes(outline!("KAT/ER"));
    assert!(!prefixes.is_empty());
}

#[test]
fn test_anscestors_descendants() {
    let mut tree = PrefixTree::new();
    tree.add(outline!("KAT"), "cat".to_string());
    tree.add(outline!("KAT/ER"), "cater".to_string());
    tree.add(outline!("KAT/ER/S"), "caters".to_string());
    tree.add(outline!("SKAT"), "scat".to_string());
    tree.add(outline!("SKAT/ER"), "scatter".to_string());
    tree.add(outline!("SKAT/ER/PWRAEUPB"), "scatterbrain".to_string());
    tree.add(outline!("TKOG/S"), "dogs".to_string());

    assert_eq!(tree.ancestors(outline!("KAT/ER/S")), vec![
        outline!("KAT/ER/S"),
        outline!("KAT/ER"),
        outline!("KAT"),
    ]);
    assert_eq!(tree.descendants(outline!("TKOG")), vec![
        outline!("TKOG/S"),
    ]);
}
