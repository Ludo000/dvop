    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic {
            message: "Test error".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 10,
            column: 5,
            end_line: Some(10),
            end_column: Some(15),
            rule: "E001".to_string(),
        };

        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 5);
    }

    #[test]
    fn test_diagnostic_severity_levels() {
        let error = DiagnosticSeverity::Error;
        let warning = DiagnosticSeverity::Warning;
        let info = DiagnosticSeverity::Info;

        assert_eq!(error, DiagnosticSeverity::Error);
        assert_eq!(warning, DiagnosticSeverity::Warning);
        assert_eq!(info, DiagnosticSeverity::Info);
    }

    #[test]
    fn test_store_and_get_file_diagnostics() {
        let path = "/test/file.rs";
        let diag = vec![Diagnostic {
            message: "Test diagnostic".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 5,
            column: 10,
            end_line: Some(5),
            end_column: Some(20),
            rule: "E001".to_string(),
        }];

        store_file_diagnostics(path, diag);
        let retrieved = get_file_diagnostics(path);

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].message, "Test diagnostic");
    }

    #[test]
    fn test_clear_file_diagnostics() {
        let path = "/test/clear.rs";
        let diag = vec![Diagnostic {
            message: "Test".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 1,
            column: 1,
            end_line: None,
            end_column: None,
            rule: "E".to_string(),
        }];

        store_file_diagnostics(path, diag);
        assert_eq!(get_file_diagnostics(path).len(), 1);

        store_file_diagnostics(path, vec![]);
        assert_eq!(get_file_diagnostics(path).len(), 0);
    }

    #[test]
    fn test_empty_file_diagnostics() {
        let path = "/test/empty.rs";
        let diagnostics = get_file_diagnostics(path);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_lint_by_language() {
        let rust_code = "fn main() { }";
        let diagnostics = lint_by_language("rust", rust_code);
        // Rust diagnostics are now handled by the extension, so this returns empty
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_lint_file_rs_defers_to_extension() {
        // Rust files return empty from the local linter —
        // real diagnostics come from the rust-diagnostics extension via LSP
        let path = Path::new("test.rs");
        let diagnostics = lint_file(path, "fn main() { let x = 5 }");
        assert!(diagnostics.is_empty(), "Local linter should return empty for .rs files");
    }

    #[test]
    fn test_lint_file_unknown_extension() {
        // Unknown file extensions should return empty (no linter registered)
        let path = Path::new("file.xyz");
        let diagnostics = lint_file(path, "some content");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnostic_with_end_position() {
        let diag = Diagnostic::new(
            DiagnosticSeverity::Warning,
            "unused variable".to_string(),
            5, 10,
            "W001".to_string(),
        ).with_end_position(5, 15);

        assert_eq!(diag.end_line, Some(5));
        assert_eq!(diag.end_column, Some(15));
        assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_diagnostic_new_constructor() {
        let diag = Diagnostic::new(
            DiagnosticSeverity::Info,
            "hint message".to_string(),
            1, 1,
            "I001".to_string(),
        );
        assert_eq!(diag.line, 1);
        assert_eq!(diag.column, 1);
        assert_eq!(diag.end_line, None);
        assert_eq!(diag.end_column, None);
        assert_eq!(diag.rule, "I001");
    }
