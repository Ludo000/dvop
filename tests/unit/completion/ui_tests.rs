    use super::*;

    // ── fuzzy_match_score tests ──────────────────────────────────

    #[test]
    fn test_exact_match_scores_highest() {
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let score = fuzzy_match_score("let", "let").unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_exact_match_case_insensitive() {
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let score = fuzzy_match_score("Let", "let").unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_prefix_match() {
        let score = fuzzy_match_score("pr", "println").unwrap();
        assert!(score >= 80, "prefix match should score >=80, got {}", score);
    }

    #[test]
    fn test_prefix_case_sensitive_bonus() {
        let exact_case = fuzzy_match_score("Hash", "HashMap").unwrap();
        let diff_case = fuzzy_match_score("hash", "HashMap").unwrap();
        assert!(
            exact_case > diff_case,
            "case-sensitive prefix ({}) should beat case-insensitive ({})",
            exact_case, diff_case
        );
    }

    #[test]
    fn test_fuzzy_subsequence() {
        // "hmap" should match "HashMap" (h, m, a, p all appear in order)
        let score = fuzzy_match_score("hmap", "HashMap");
        assert!(score.is_some(), "hmap should fuzzy-match HashMap");
    }

    #[test]
    fn test_fuzzy_camelcase_bonus() {
        // Matching word boundaries (capital H, M) should get bonuses
        let score = fuzzy_match_score("HM", "HashMap").unwrap();
        assert!(score > 60, "CamelCase boundary match should score well, got {}", score);
    }

    #[test]
    fn test_no_match_returns_none() {
        assert!(fuzzy_match_score("xyz", "let").is_none());
        assert!(fuzzy_match_score("abc", "HashMap").is_none());
    }

    #[test]
    fn test_empty_query_matches_everything() {
        assert_eq!(fuzzy_match_score("", "anything").unwrap(), 0);
    }

    #[test]
    fn test_prefix_beats_fuzzy() {
        let prefix = fuzzy_match_score("pr", "println").unwrap();
        let fuzzy = fuzzy_match_score("pl", "println").unwrap();
        assert!(
            prefix >= fuzzy,
            "prefix match ({}) should beat or equal fuzzy ({})",
            prefix, fuzzy
        );
    }

    #[test]
    fn test_longer_prefix_scores_same_base() {
        let short = fuzzy_match_score("p", "println").unwrap();
        let long = fuzzy_match_score("print", "println").unwrap();
        // Both are prefix matches, should both be >= 80
        assert!(short >= 80);
        assert!(long >= 80);
    }

    // ── analyse_cursor_context tests ─────────────────────────────

    #[test]
    fn test_context_type_position_colon() {
        assert_eq!(analyse_cursor_context("let x: "), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_type_position_arrow() {
        assert_eq!(analyse_cursor_context("fn foo() -> "), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_type_position_generic() {
        assert_eq!(analyse_cursor_context("Vec<"), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_statement_start_empty() {
        assert_eq!(analyse_cursor_context(""), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_statement_start_brace() {
        assert_eq!(analyse_cursor_context("fn main() {"), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_statement_start_semicolon() {
        assert_eq!(analyse_cursor_context("let x = 5;"), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_expression_equals() {
        assert_eq!(analyse_cursor_context("let x = "), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_expression_paren() {
        assert_eq!(analyse_cursor_context("foo("), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_expression_comma() {
        assert_eq!(analyse_cursor_context("foo(a, "), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_dot_access() {
        assert_eq!(analyse_cursor_context("my_var."), CursorContext::DotAccess);
    }

    #[test]
    fn test_context_newline_is_statement_start() {
        assert_eq!(analyse_cursor_context("let x = 5;\n    "), CursorContext::StatementStart);
    }

    // ── context_bonus tests ──────────────────────────────────────

    #[test]
    fn test_type_keyword_boosted_in_type_position() {
        let item = CompletionItem::Keyword("String".to_string());
        let bonus = context_bonus(&item, CursorContext::TypePosition, Some("type"), None);
        assert!(bonus > 0, "type keyword should get positive bonus in type position");
    }

    #[test]
    fn test_control_flow_demoted_in_type_position() {
        let item = CompletionItem::Keyword("for".to_string());
        let bonus = context_bonus(
            &item,
            CursorContext::TypePosition,
            Some("keyword"),
            Some("control_flow"),
        );
        assert!(bonus < 0, "control_flow keyword should be demoted in type position");
    }

    #[test]
    fn test_snippet_boosted_at_statement_start() {
        let item = CompletionItem::Snippet("fn".to_string(), "fn ${1:name}()".to_string());
        let bonus = context_bonus(&item, CursorContext::StatementStart, None, None);
        assert!(bonus > 0, "snippets should be boosted at statement start");
    }

    #[test]
    fn test_buffer_word_boosted_in_expression() {
        let item = CompletionItem::BufferWord("my_var".to_string());
        let bonus = context_bonus(&item, CursorContext::ExpressionPosition, None, None);
        assert!(bonus > 0, "buffer words should be boosted in expression position");
    }

    // ── completion_item_name tests ───────────────────────────────

    #[test]
    fn test_completion_item_name_keyword() {
        assert_eq!(completion_item_name(&CompletionItem::Keyword("let".to_string())), "let");
    }

    #[test]
    fn test_completion_item_name_snippet() {
        assert_eq!(
            completion_item_name(&CompletionItem::Snippet("fn".to_string(), "content".to_string())),
            "fn"
        );
    }

    #[test]
    fn test_completion_item_name_buffer_word() {
        assert_eq!(
            completion_item_name(&CompletionItem::BufferWord("my_var".to_string())),
            "my_var"
        );
    }
