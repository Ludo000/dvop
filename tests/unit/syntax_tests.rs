    use super::*;

    #[test]
    fn test_get_preferred_style_scheme() {
        let scheme = get_preferred_style_scheme();
        // Should return a valid scheme name
        assert!(!scheme.is_empty());
    }
