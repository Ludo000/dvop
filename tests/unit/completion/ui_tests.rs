    use super::*;
    use gtk4::prelude::*;
    use serial_test::serial;

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

    #[test]
    fn test_context_general_for_mid_expression_text() {
        assert_eq!(analyse_cursor_context("foo bar "), CursorContext::General);
    }

    #[test]
    fn test_general_context_bonus_is_neutral() {
        let item = CompletionItem::Snippet("foo".to_string(), "foo()".to_string());
        let bonus = context_bonus(&item, CursorContext::General, None, None);
        assert_eq!(bonus, 0);
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

    #[test]
    fn expand_snippet_content_replaces_placeholder_defaults() {
        assert_eq!(expand_snippet_content("fn ${1:name}() {}"), "fn name() {}");
        assert_eq!(expand_snippet_content("let ${1} = ${2:value};"), "let placeholder = value;");
    }

    #[test]
    fn expand_snippet_content_uses_generic_placeholder_for_numeric_only_markers() {
        assert_eq!(expand_snippet_content("item ${2}"), "item placeholder");
    }

    #[test]
    fn expand_snippet_content_preserves_literal_dollar_braces_without_placeholders() {
        assert_eq!(expand_snippet_content("cost is $5"), "cost is $5");
    }

    #[test]
    fn detect_import_context_recognizes_rust_and_script_imports() {
        assert!(detect_import_context("use std::fmt::"));
        assert!(detect_import_context("import foo from 'lodash';"));
        assert!(detect_import_context("from pathlib import Path"));
        assert!(!detect_import_context("let total = 0;"));
    }

    #[test]
    fn detect_import_context_recognizes_commonjs_require_calls() {
        assert!(detect_import_context("const fs = require('fs');"));
    }

    #[test]
    fn detect_import_context_recognizes_python_bare_import_statement() {
        assert!(detect_import_context("import os"));
    }

    #[test]
    fn extract_import_path_parses_python_dotted_import() {
        assert_eq!(
            extract_import_path("import django.utils"),
            Some("django".to_string())
        );
    }

    #[test]
    fn extract_import_path_parses_rust_module_prefix() {
        assert_eq!(
            extract_import_path("use std::fmt::"),
            Some("std::fmt".to_string())
        );
        assert_eq!(
            extract_import_path("use std::"),
            Some("std".to_string())
        );
    }

    #[test]
    fn extract_import_path_parses_javascript_module_string() {
        assert_eq!(
            extract_import_path("import React from 'react';"),
            Some("react".to_string())
        );
    }

    #[test]
    fn extract_import_path_parses_python_from_import() {
        assert_eq!(
            extract_import_path("from collections import defaultdict"),
            Some("collections".to_string())
        );
    }

    #[test]
    fn analyse_cursor_context_treats_return_keyword_as_expression_position() {
        assert_eq!(analyse_cursor_context("return "), CursorContext::ExpressionPosition);
    }

    #[test]
    fn expand_snippet_content_handles_nested_braces_in_placeholder() {
        assert_eq!(
            expand_snippet_content("wrap ${1:outer ${2:inner}} end"),
            "wrap outer ${2:inner} end"
        );
    }

    #[test]
    fn extract_import_path_returns_none_for_non_import_context() {
        assert_eq!(extract_import_path("let total = 0;"), None);
    }

    #[test]
    fn context_bonus_boosts_import_functions_in_expression_position() {
        let item = CompletionItem::ImportItem(ImportItem {
            name: "read".to_string(),
            item_type: "function".to_string(),
            description: "Reads bytes".to_string(),
        });
        let bonus = context_bonus(&item, CursorContext::ExpressionPosition, None, None);
        assert_eq!(bonus, 20);
    }

    #[test]
    fn context_bonus_boosts_struct_import_in_type_position() {
        let item = CompletionItem::ImportItem(ImportItem {
            name: "HashMap".to_string(),
            item_type: "struct".to_string(),
            description: "Map type".to_string(),
        });
        let bonus = context_bonus(&item, CursorContext::TypePosition, None, None);
        assert_eq!(bonus, 25);
    }

    #[test]
    fn context_bonus_demotes_non_type_keywords_in_type_position() {
        let item = CompletionItem::Keyword("let".to_string());
        let bonus = context_bonus(&item, CursorContext::TypePosition, Some("keyword"), None);
        assert_eq!(bonus, -10);
    }

    #[test]
    fn context_bonus_boosts_snippets_at_statement_start() {
        let item = CompletionItem::Snippet("fn".to_string(), "fn ${1:name}() {}".to_string());
        let bonus = context_bonus(&item, CursorContext::StatementStart, None, None);
        assert_eq!(bonus, 15);
    }

    #[test]
    fn completion_item_name_returns_import_module_path() {
        assert_eq!(
            completion_item_name(&CompletionItem::ImportModule("std::fmt".to_string())),
            "std::fmt"
        );
    }

    #[test]
    #[serial]
    fn get_buffer_language_maps_typescript_to_javascript_pack() {
        gtk4::test_synced(|| {
            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            let manager = sourceview5::LanguageManager::new();
            if let Some(language) = manager.language("typescript") {
                buffer.set_language(Some(&language));
                assert_eq!(get_buffer_language(&buffer), "javascript");
            }
        });
    }

    #[test]
    #[serial]
    fn get_buffer_language_defaults_to_supported_language_without_buffer_language() {
        gtk4::test_synced(|| {
            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            buffer.set_language(None::<&sourceview5::Language>);
            let detected = get_buffer_language(&buffer);
            assert!(crate::completion::get_supported_languages().contains(&detected));
        });
    }

    #[test]
    #[serial]
    fn get_buffer_language_maps_javascript_language_ids() {
        gtk4::test_synced(|| {
            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            let manager = sourceview5::LanguageManager::new();
            for lang_id in ["javascript", "js"] {
                if let Some(language) = manager.language(lang_id) {
                    buffer.set_language(Some(&language));
                    assert_eq!(get_buffer_language(&buffer), "javascript");
                }
            }
        });
    }
