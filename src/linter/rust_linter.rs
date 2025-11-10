// Rust-specific linter using the syn crate
// This module provides linting functionality for Rust source code

use super::{Diagnostic, DiagnosticSeverity};
use std::collections::HashSet;
use syn::{visit::Visit, Expr, Ident, ItemFn, Local, Pat};

/// Visitor pattern implementation for collecting lint diagnostics
struct RustLintVisitor {
    diagnostics: Vec<Diagnostic>,
    declared_variables: HashSet<String>,
    used_variables: HashSet<String>,
}

impl RustLintVisitor {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            declared_variables: HashSet::new(),
            used_variables: HashSet::new(),
        }
    }

    fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn check_naming_convention(&mut self, ident: &Ident, expected_style: &str) {
        let name = ident.to_string();
        let is_snake_case = name
            .chars()
            .all(|c| c.is_lowercase() || c.is_numeric() || c == '_');
        let is_upper_case = name
            .chars()
            .all(|c| c.is_uppercase() || c.is_numeric() || c == '_');
        let is_camel_case =
            name.chars().next().map_or(false, |c| c.is_uppercase()) && !name.contains('_');

        let valid = match expected_style {
            "snake_case" => is_snake_case,
            "SCREAMING_SNAKE_CASE" => is_upper_case,
            "CamelCase" => is_camel_case,
            _ => true,
        };

        if !valid {
            // Note: syn's Span doesn't provide line/column info directly
            // We'll use line 0 as a placeholder - for production use, you'd need
            // to track positions differently or use proc-macro2 features

            self.add_diagnostic(Diagnostic::new(
                DiagnosticSeverity::Warning,
                format!(
                    "Identifier '{}' should be in {} format",
                    name, expected_style
                ),
                0, // Placeholder line number
                0, // Placeholder column number
                "naming_convention".to_string(),
            ));
        }
    }
}

impl<'ast> Visit<'ast> for RustLintVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Check function naming convention (should be snake_case)
        self.check_naming_convention(&node.sig.ident, "snake_case");

        // Continue visiting the function body
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_local(&mut self, node: &'ast Local) {
        // Track variable declarations
        if let Pat::Ident(pat_ident) = &node.pat {
            let var_name = pat_ident.ident.to_string();
            self.declared_variables.insert(var_name.clone());

            // Check variable naming convention (should be snake_case)
            self.check_naming_convention(&pat_ident.ident, "snake_case");
        }

        syn::visit::visit_local(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        // Track variable usage
        if let Expr::Path(expr_path) = node {
            if let Some(ident) = expr_path.path.get_ident() {
                self.used_variables.insert(ident.to_string());
            }
        }

        syn::visit::visit_expr(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        // Check struct naming convention (should be CamelCase)
        self.check_naming_convention(&node.ident, "CamelCase");

        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        // Check enum naming convention (should be CamelCase)
        self.check_naming_convention(&node.ident, "CamelCase");

        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        // Check constant naming convention (should be SCREAMING_SNAKE_CASE)
        self.check_naming_convention(&node.ident, "SCREAMING_SNAKE_CASE");

        syn::visit::visit_item_const(self, node);
    }
}

/// Lint Rust source code and return diagnostics
pub fn lint_rust_code(code: &str) -> Vec<Diagnostic> {
    // Try to parse the code
    let syntax_tree = match syn::parse_file(code) {
        Ok(tree) => tree,
        Err(e) => {
            // Return a syntax error diagnostic
            return vec![Diagnostic::new(
                DiagnosticSeverity::Error,
                format!("Syntax error: {}", e),
                0, // syn doesn't provide easy access to error position
                0,
                "syntax_error".to_string(),
            )];
        }
    };

    // Create visitor and collect diagnostics
    let mut visitor = RustLintVisitor::new();
    visitor.visit_file(&syntax_tree);

    // Check for unused variables - collect them first to avoid borrow issues
    let unused_vars: Vec<String> = visitor
        .declared_variables
        .iter()
        .filter(|var| !visitor.used_variables.contains(*var) && !var.starts_with('_'))
        .cloned()
        .collect();

    for var in unused_vars {
        visitor.add_diagnostic(Diagnostic::new(
            DiagnosticSeverity::Warning,
            format!("Variable '{}' is declared but never used", var),
            0, // We'd need to track the declaration location for accurate line numbers
            0,
            "unused_variable".to_string(),
        ));
    }

    // Additional checks can be added here:
    // - Dead code detection
    // - Complexity metrics
    // - Style violations
    // - Performance hints

    visitor.diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_convention_function() {
        let code = r#"
            fn BadFunctionName() {
                let x = 1;
            }
        "#;

        let diagnostics = lint_rust_code(code);
        assert!(diagnostics.iter().any(|d| d.rule == "naming_convention"));
    }

    #[test]
    fn test_naming_convention_struct() {
        let code = r#"
            struct bad_struct_name {
                field: i32,
            }
        "#;

        let diagnostics = lint_rust_code(code);
        assert!(diagnostics.iter().any(|d| d.rule == "naming_convention"));
    }

    #[test]
    fn test_valid_code() {
        let code = r#"
            fn good_function_name() {
                let variable_name = 1;
            }
        "#;

        let diagnostics = lint_rust_code(code);
        assert!(!diagnostics.iter().any(|d| d.rule == "naming_convention"));
    }

    #[test]
    fn test_syntax_error() {
        let code = "fn broken() { let x = ";

        let diagnostics = lint_rust_code(code);
        assert!(diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error));
    }
}
