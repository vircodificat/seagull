#[cfg(test)]
mod prefix_tree_tests {
    use crate::{Outline, PrefixTree, prefix_tree_from_json_dictionary, JsonDictionary};
    use std::collections::HashMap;

    fn test_tree() -> PrefixTree {
        let mut map: HashMap<Outline, String> = HashMap::new();
        map.insert(Outline::try_from_string("KAT").unwrap(), "cat".to_string());
        map.insert(Outline::try_from_string("KAT/ER").unwrap(), "cater".to_string());
        map.insert(Outline::try_from_string("KAT/ER/S").unwrap(), "caters".to_string());
        map.insert(Outline::try_from_string("SKAT").unwrap(), "scat".to_string());
        map.insert(Outline::try_from_string("SKAT/ER").unwrap(), "scatter".to_string());
        map.insert(Outline::try_from_string("SKAT/ER/PWRAEUPB").unwrap(), "scatterbrain".to_string());
        PrefixTree::new(map)
    }

    #[test]
    #[ignore]
    fn test_contains() {
        let tree = test_tree();
        assert!(tree.contains(Outline::try_from_string("KAT").unwrap()));
        assert!(tree.contains(Outline::try_from_string("KAT/ER").unwrap()));
        assert!(tree.contains(Outline::try_from_string("KAT/ER/S").unwrap()));
        assert!(!tree.contains(Outline::try_from_string("KAT/XYZ").unwrap()));
        assert!(!tree.contains(Outline::try_from_string("XYZ").unwrap()));
    }

    #[test]
    #[ignore]
    fn test_following_strokes() {
        let tree = test_tree();
        let following = tree.following_strokes(Outline::try_from_string("KAT").unwrap());
        assert!(following.contains(&crate::Stroke::try_from_string("ER").unwrap()));
        
        let following_empty = tree.following_strokes(Outline::try_from_string("KAT/ER/S").unwrap());
        assert!(following_empty.is_empty());
        
        let following_none = tree.following_strokes(Outline::try_from_string("KAT/XYZ").unwrap());
        assert!(following_none.is_empty());
    }

    #[test]
    #[ignore]
    fn test_prefix_strokes() {
        let tree = test_tree();
        
        let prefixes = tree.prefix_strokes(Outline::try_from_string("KAT/ER").unwrap());
        assert_eq!(prefixes.len(), 1);
        assert_eq!(prefixes[0], crate::Stroke::try_from_string("KAT").unwrap());
        
        let prefixes_multi = tree.prefix_strokes(Outline::try_from_string("SKAT/ER/PWRAEUPB").unwrap());
        assert_eq!(prefixes_multi.len(), 2);
        
        let no_prefix = tree.prefix_strokes(Outline::try_from_string("KAT/XYZ").unwrap());
        assert!(no_prefix.is_empty());
    }

    #[test]
    #[ignore]
    fn test_lookup() {
        let tree = test_tree();
        assert_eq!(tree.lookup(Outline::try_from_string("KAT").unwrap()), Some("cat".to_string()));
        assert_eq!(tree.lookup(Outline::try_from_string("KAT/ER").unwrap()), Some("cater".to_string()));
        assert_eq!(tree.lookup(Outline::try_from_string("KAT/ER/S").unwrap()), Some("caters".to_string()));
        assert_eq!(tree.lookup(Outline::try_from_string("XYZ").unwrap()), None);
    }

    #[test]
    #[ignore]
    fn test_from_main_json() {
        let dictionary = JsonDictionary::load_from_file("../data/main.json").unwrap();
        let tree = prefix_tree_from_json_dictionary(dictionary);
        
        assert!(tree.contains(Outline::try_from_string("KAT").unwrap()));
        assert!(tree.contains(Outline::try_from_string("KAT/ER").unwrap()));
        
        let following = tree.following_strokes(Outline::try_from_string("KAT").unwrap());
        assert!(!following.is_empty());
        
        let prefixes = tree.prefix_strokes(Outline::try_from_string("KAT/ER").unwrap());
        assert!(!prefixes.is_empty());
    }
}
