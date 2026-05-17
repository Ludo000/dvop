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
