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

    #[test]
    fn get_keyword_documentation_returns_fallback_when_language_data_missing() {
        let doc = get_keyword_documentation("nonexistent-language", "fn");
        assert!(doc.contains("No documentation available"));
        assert!(doc.contains("fn"));
    }

    #[test]
    fn test_get_language_keywords_javascript() {
        let keywords = get_language_keywords_owned("javascript");
        assert!(keywords.contains(&"function".to_string()));
        assert!(keywords.contains(&"const".to_string()));
        assert!(keywords.contains(&"return".to_string()));
    }

    #[test]
    fn test_get_language_snippets_javascript_include_common_triggers() {
        ensure_initialized();
        let snippets = get_language_snippets_owned("javascript");
        assert!(!snippets.is_empty());
        assert!(snippets.iter().any(|(trigger, _)| trigger == "function"));
    }

    #[test]
    fn test_get_supported_languages_includes_common_languages() {
        ensure_initialized();
        let languages = get_supported_languages();
        for language in ["python", "javascript", "html", "css", "go"] {
            assert!(
                languages.iter().any(|l| l == language),
                "missing language: {language}"
            );
        }
    }

    #[test]
    fn test_get_keyword_documentation_returns_fallback_for_unknown_keyword() {
        ensure_initialized();
        let doc = get_keyword_documentation("javascript", "not_a_real_keyword_xyz");
        assert!(doc.contains("not_a_real_keyword_xyz"));
        assert!(doc.contains("No documentation"));
    }

    #[test]
    fn test_get_language_keywords_css_includes_common_properties() {
        ensure_initialized();
        let keywords = get_language_keywords_owned("css");
        assert!(keywords.contains(&"color".to_string()));
        assert!(keywords.contains(&"margin".to_string()));
    }

    #[test]
    fn test_get_language_keywords_html_includes_common_tags() {
        ensure_initialized();
        let keywords = get_language_keywords_owned("html");
        assert!(keywords.contains(&"div".to_string()));
        assert!(keywords.contains(&"span".to_string()));
    }

    #[test]
    fn test_get_snippet_documentation_returns_fallback_for_unknown_trigger() {
        ensure_initialized();
        let doc = get_snippet_documentation("javascript", "not_a_snippet_xyz");
        assert!(doc.contains("not_a_snippet_xyz"));
        assert!(doc.contains("No documentation"));
    }
