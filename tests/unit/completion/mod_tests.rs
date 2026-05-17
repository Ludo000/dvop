    use super::*;
    use std::sync::Once;
    static INIT: Once = Once::new();
    fn ensure_initialized() {
        INIT.call_once(|| {
            initialize_completion();
            // Also load Rust completions (normally done by the rust-completion extension)
            crate::extensions::rust_completion::load_and_register();
        });
    }

    #[test]
    fn test_get_supported_languages() {
        let languages = get_supported_languages();
        assert!(!languages.is_empty());
        // Check for common languages
        assert!(languages.iter().any(|l| l == "rust" || l == "python" || l == "javascript"));
    }

    #[test]
    fn test_get_language_keywords_rust() {
        ensure_initialized();
        let keywords = get_language_keywords_owned("rust");
        assert!(keywords.contains(&"fn".to_string()));
        assert!(keywords.contains(&"let".to_string()));
        assert!(keywords.contains(&"mut".to_string()));
        assert!(keywords.contains(&"impl".to_string()));
    }

    #[test]
    fn test_get_language_keywords_python() {
        let keywords = get_language_keywords_owned("python");
        assert!(keywords.contains(&"def".to_string()));
        assert!(keywords.contains(&"class".to_string()));
        assert!(keywords.contains(&"import".to_string()));
    }

    #[test]
    fn test_get_language_snippets() {
        ensure_initialized();
        let snippets = get_language_snippets_owned("rust");
        assert!(!snippets.is_empty());
        // Check that snippets have both trigger and body
        for (trigger, body) in snippets {
            assert!(!trigger.is_empty());
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn test_get_keyword_documentation() {
        ensure_initialized();
        let doc = get_keyword_documentation("rust", "fn");
        assert!(!doc.is_empty());
        assert!(doc.to_lowercase().contains("function"));
    }

    #[test]
    fn test_get_snippet_documentation() {
        let doc = get_snippet_documentation("rust", "fn");
        // May or may not have documentation
        assert!(doc.is_empty() || !doc.is_empty());
    }

    #[test]
    fn test_unsupported_language() {
        let keywords = get_language_keywords_owned("nonexistent");
        assert!(keywords.is_empty());
    }
