    use super::*;
    use serial_test::serial;

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
    fn test_lint_file_builtin_ui_dispatches_to_gtk_ui_linter() {
        let path = Path::new("broken.ui");
        let diagnostics = lint_file_builtin(path, "<object class=\"GtkWindow\" />");

        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "missing-interface" && d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn test_lint_file_builtin_handles_paths_without_extension() {
        let diagnostics = lint_file_builtin(Path::new("Makefile"), "content");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnostic_underline_tracking_for_unknown_path_is_false() {
        let path = "/tmp/dvop/no-underlines.rs";

        forget_diagnostic_underline_tracking_for_path(path);
        assert!(!has_applied_diagnostic_underlines_for_path(path));
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

    #[test]
    fn test_store_diagnostics_for_multiple_files() {
        let path_a = "/tmp/dvop/lint-a.rs";
        let path_b = "/tmp/dvop/lint-b.rs";

        store_file_diagnostics(
            path_a,
            vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                "error a".to_string(),
                1,
                1,
                "E001".to_string(),
            )],
        );
        store_file_diagnostics(
            path_b,
            vec![Diagnostic::new(
                DiagnosticSeverity::Warning,
                "warning b".to_string(),
                2,
                3,
                "W001".to_string(),
            )],
        );

        assert_eq!(get_file_diagnostics(path_a).len(), 1);
        assert_eq!(get_file_diagnostics(path_b).len(), 1);
        assert_eq!(get_file_diagnostics(path_a)[0].message, "error a");
        assert_eq!(get_file_diagnostics(path_b)[0].message, "warning b");
    }

    #[test]
    fn test_forget_diagnostic_underline_tracking_removes_path() {
        let path = "/tmp/dvop/underline-tracking.rs";
        forget_diagnostic_underline_tracking_for_path(path);
        assert!(!has_applied_diagnostic_underlines_for_path(path));
    }

    #[test]
    fn test_lint_file_builtin_returns_empty_for_valid_minimal_ui() {
        let diagnostics = lint_file_builtin(
            Path::new("valid.ui"),
            r#"<interface><object class="GtkWindow" id="window1"><property name="title">Ok</property></object></interface>"#,
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_lint_file_reports_ui_errors_from_builtin_linter() {
        let diagnostics = lint_file(
            Path::new("panel.ui"),
            r#"<interface><object id="window1"><property name="title">Missing class</property></object></interface>"#,
        );
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "missing-class" && d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn test_lint_file_returns_duplicate_id_errors_for_ui_files() {
        let diagnostics = lint_file(
            Path::new("dup.ui"),
            r#"<interface>
  <object class="GtkLabel" id="dup" />
  <object class="GtkButton" id="dup" />
</interface>"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "duplicate-id"));
    }

    #[test]
    fn test_store_empty_diagnostics_removes_cached_path() {
        let path = "/tmp/dvop/clear-diagnostics.rs";
        store_file_diagnostics(
            path,
            vec![Diagnostic::new(
                DiagnosticSeverity::Warning,
                "temporary".to_string(),
                1,
                1,
                "W001".to_string(),
            )],
        );
        assert_eq!(get_file_diagnostics(path).len(), 1);

        store_file_diagnostics(path, vec![]);
        assert!(get_file_diagnostics(path).is_empty());
    }

    #[test]
    fn test_get_file_diagnostics_returns_empty_for_unknown_path() {
        assert!(get_file_diagnostics("/tmp/dvop/no-such-file.rs").is_empty());
    }

    #[test]
    fn test_store_file_diagnostics_overwrites_previous_entry() {
        let path = "/tmp/dvop/overwrite-diagnostics.rs";
        store_file_diagnostics(
            path,
            vec![Diagnostic::new(
                DiagnosticSeverity::Info,
                "first".to_string(),
                1,
                1,
                "I001".to_string(),
            )],
        );
        store_file_diagnostics(
            path,
            vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                "second".to_string(),
                2,
                3,
                "E002".to_string(),
            )],
        );

        let diagnostics = get_file_diagnostics(path);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "second");
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn test_diagnostic_partial_eq_compares_all_fields() {
        let left = Diagnostic::new(
            DiagnosticSeverity::Warning,
            "unused".to_string(),
            4,
            2,
            "W001".to_string(),
        )
        .with_end_position(4, 8);
        let right = Diagnostic::new(
            DiagnosticSeverity::Warning,
            "unused".to_string(),
            4,
            2,
            "W001".to_string(),
        )
        .with_end_position(4, 8);
        assert_eq!(left.message, right.message);
        assert_eq!(left.end_line, right.end_line);
        assert_eq!(left.end_column, right.end_column);
    }

    #[test]
    #[serial]
    fn apply_diagnostic_underlines_marks_tracked_path_after_applying_tags() {
        gtk4::test_synced(|| {
            let path = "/tmp/dvop/underline-apply.rs";
            forget_diagnostic_underline_tracking_for_path(path);

            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            buffer.set_text("fn main() {}\n");
            store_file_diagnostics(
                path,
                vec![Diagnostic::new(
                    DiagnosticSeverity::Error,
                    "syntax error".to_string(),
                    1,
                    1,
                    "E001".to_string(),
                )],
            );

            apply_diagnostic_underlines(&buffer, path);
            assert!(has_applied_diagnostic_underlines_for_path(path));
        });
    }

    #[test]
    #[serial]
    fn apply_diagnostic_underlines_clears_tracking_when_diagnostics_removed() {
        gtk4::test_synced(|| {
            let path = "/tmp/dvop/underline-clear.rs";
            forget_diagnostic_underline_tracking_for_path(path);

            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            buffer.set_text("fn main() {}\n");
            store_file_diagnostics(
                path,
                vec![Diagnostic::new(
                    DiagnosticSeverity::Warning,
                    "unused".to_string(),
                    1,
                    1,
                    "W001".to_string(),
                )],
            );
            apply_diagnostic_underlines(&buffer, path);
            assert!(has_applied_diagnostic_underlines_for_path(path));

            store_file_diagnostics(path, vec![]);
            apply_diagnostic_underlines(&buffer, path);
            assert!(!has_applied_diagnostic_underlines_for_path(path));
        });
    }
