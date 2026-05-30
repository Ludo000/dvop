    use super::*;

    #[test]
    fn test_rust_analyzer_config() {
        let config = LanguageServerConfig::rust_analyzer();
        assert_eq!(config.name, "rust-analyzer");
        assert_eq!(config.command, "rust-analyzer");
        assert_eq!(config.file_extensions, vec!["rs"]);
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_lsp_diagnostic_conversion_error() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 15,
                },
            },
            severity: Some(LspSeverity::ERROR),
            code: None,
            message: "Test error".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Error);
        assert_eq!(diag.line, 11); // 1-indexed
        assert_eq!(diag.column, 6); // 1-indexed
    }

    #[test]
    fn test_lsp_diagnostic_conversion_warning() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 5, character: 10 },
                end: Position { line: 5, character: 20 },
            },
            severity: Some(LspSeverity::WARNING),
            code: None,
            message: "Test warning".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_lsp_diagnostic_conversion_info() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            severity: Some(LspSeverity::INFORMATION),
            code: None,
            message: "Test info".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Info);
    }

    #[test]
    fn test_lsp_diagnostic_with_code() {
        use lsp_types::{Diagnostic as LspDiagnostic, NumberOrString, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            },
            severity: Some(LspSeverity::ERROR),
            code: Some(NumberOrString::String("E0425".to_string())),
            message: "Cannot find value".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.message, "Cannot find value");
    }

    #[test]
    fn test_lsp_diagnostic_conversion_hint_maps_to_info() {
        use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 3, character: 0 },
                end: Position { line: 3, character: 4 },
            },
            severity: Some(LspSeverity::HINT),
            code: None,
            message: "consider renaming".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Info);
        assert_eq!(diag.line, 4);
    }

    #[test]
    fn test_lsp_diagnostic_conversion_numeric_code() {
        use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity, NumberOrString, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 1 },
            },
            severity: Some(LspSeverity::ERROR),
            code: Some(NumberOrString::Number(42)),
            message: "numeric code".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.rule, "42");
    }

    #[test]
    fn test_lsp_diagnostic_conversion_defaults_missing_severity_to_info() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 8, character: 2 },
                end: Position { line: 8, character: 5 },
            },
            severity: None,
            code: None,
            message: "unspecified severity".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Info);
        assert_eq!(diag.rule, "lsp_diagnostic");
    }

    #[test]
    fn test_lsp_diagnostic_conversion_preserves_end_position() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 2, character: 4 },
                end: Position { line: 2, character: 12 },
            },
            severity: Some(LspSeverity::WARNING),
            code: None,
            message: "span".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.line, 3);
        assert_eq!(diag.column, 5);
        assert_eq!(diag.end_line, Some(3));
        assert_eq!(diag.end_column, Some(13));
    }

    #[test]
    fn test_language_server_config_rust_analyzer_is_cloneable() {
        let config = LanguageServerConfig::rust_analyzer();
        let cloned = config.clone();
        assert_eq!(cloned.name, "rust-analyzer");
        assert_eq!(cloned.file_extensions, vec!["rs"]);
    }

    #[test]
    fn test_language_server_config_debug_includes_name() {
        let config = LanguageServerConfig::rust_analyzer();
        let debug = format!("{config:?}");
        assert!(debug.contains("rust-analyzer"));
    }

    #[test]
    fn test_language_server_config_rust_analyzer_targets_rust_only() {
        let config = LanguageServerConfig::rust_analyzer();
        assert_eq!(config.command, "rust-analyzer");
        assert!(config.args.is_empty());
        assert_eq!(config.file_extensions, vec!["rs"]);
    }

    #[test]
    fn test_lsp_diagnostic_conversion_preserves_empty_message() {
        use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(LspSeverity::WARNING),
            code: None,
            message: String::new(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert!(diag.message.is_empty());
        assert_eq!(diag.end_line, Some(1));
        assert_eq!(diag.end_column, Some(1));
    }

    #[test]
    fn test_lsp_diagnostic_without_severity_defaults_to_info() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 2, character: 4 },
                end: Position { line: 2, character: 8 },
            },
            severity: None,
            code: None,
            message: "unspecified".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Info);
        assert_eq!(diag.rule, "lsp_diagnostic");
    }

    #[test]
    fn test_lsp_diagnostic_conversion_maps_numeric_code_to_rule() {
        use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity, NumberOrString, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 1 },
            },
            severity: Some(LspSeverity::ERROR),
            code: Some(NumberOrString::Number(42)),
            message: "numbered".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.rule, "42");
    }
