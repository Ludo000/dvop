# Dvop End-to-End Testing

## Overview

Dvop has comprehensive E2E tests that deeply validate each feature's functionality. Unlike the previous shallow tests, these tests:

- **Create actual files and workspaces**
- **Test real user workflows** (edit, save, search, replace, lint, etc.)
- **Verify feature behavior** not just initialization
- **Use tempdir for isolated testing**

## Test Structure

- **tests/e2e_tests.rs**: 21+ deep E2E tests covering core features
- **tests/comprehensive_tests.rs**: 192 individual smoke tests (one per FEATURES.md entry)
- **tests/quick_tests.rs**: Fast integration tests for basic functionality

## Running E2E Tests

### Individual Test (Recommended)
```bash
# Run a specific test
cargo test --test e2e_tests test_feature_001_multi_tab_editing_deep

# Run with output
cargo test --test e2e_tests test_feature_040_rust_linting_deep -- --nocapture
```

### All E2E Tests (Using Script)
```bash
# Runs each test individually to avoid GTK threading issues
./run_e2e_tests.sh
```

### Quick Test (Accepts GTK failures)
```bash
# Runs all together - some may fail due to GTK threading
cargo test --test e2e_tests
```

## GTK Threading Limitation

**IMPORTANT**: E2E tests create GTK/GtkSourceView widgets, which MUST be created on the main thread. Rust's test framework runs each test in a separate thread, causing failures when multiple tests run together.

**Solutions**:
1. ✅ **Run tests individually** (use `./run_e2e_tests.sh`)
2. ❌ `--test-threads=1` doesn't help (still separate threads, just sequential)
3. ❌ `#[serial]` doesn't help (queues but still separate threads)

## Test Categories

### Text Editor (Features 1-17)
- **test_feature_001_multi_tab_editing_deep**: Creates multiple tabs, switches between them, tests closing
- **test_feature_002_syntax_highlighting_deep**: Tests Rust, Python, JS language detection and keyword highlighting
- **test_feature_003_line_numbers_deep**: Verifies line numbers, multiline text, toggle visibility
- **test_feature_004_cursor_position_tracking_deep**: Tests cursor movement, position tracking
- **test_feature_005_auto_indentation_deep**: Validates tab width, indentation settings
- **test_feature_007_undo_redo_deep**: Tests undo/redo functionality
- **test_feature_008_search_replace_basic**: Tests search and replace operations
- **test_feature_009_save_load_file**: Tests file I/O, persistence

### File Management (Features 18-35)
- **test_feature_018_file_explorer_deep**: Tests directory listing, file/folder detection
- **test_feature_021_create_new_file**: Creates files programmatically
- **test_feature_022_delete_file**: Tests file deletion
- **test_feature_023_rename_file**: Tests file renaming

### Code Intelligence (Features 36-53)
- **test_feature_036_autocompletion_deep**: Tests keyword completion for Rust, Python, JS
- **test_feature_040_rust_linting_deep**: Tests Rust linter with valid/invalid code
- **test_feature_041_diagnostics_panel_deep**: Tests diagnostics UI creation

### Search (Features 54-64)
- **test_feature_054_find_in_file_deep**: Tests in-file search, case sensitivity
- **test_feature_055_replace_in_file_deep**: Tests replace functionality
- **test_feature_058_global_search_deep**: Tests cross-file search

### Other
- **test_feature_065_embedded_terminal_creation**: Terminal widget creation
- **test_feature_075_git_status_detection**: Git integration testing

## Test Coverage

| Category | E2E Tests | Coverage |
|----------|-----------|----------|
| Text Editor | 8 tests | Deep testing of core editing features |
| File Management | 4 tests | Full CRUD operations |
| Code Intelligence | 3 tests | Completion, linting, diagnostics |
| Search | 3 tests | Find, replace, global search |
| Git | 1 test | Status detection |
| Terminal | 1 test | Widget creation |
| **Total** | **21 tests** | **Core feature validation** |

## Extending Tests

To add more E2E tests:

1. Add test function to `tests/e2e_tests.rs`
2. Use `#[serial]` and `#[test]` attributes
3. Call `init_gtk()` at start
4. Use `create_test_workspace()` for file operations
5. Test actual functionality, not just initialization
6. Run individually to verify it passes

Example:
```rust
#[serial]
#[test]
fn test_feature_XXX_my_feature() {
    init_gtk();
    let workspace = create_test_workspace();
    
    // Test real functionality
    let (view, buffer) = dvop::syntax::create_source_view();
    buffer.set_text("test content");
    
    assert_eq!(buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).as_str(), "test content");
}
```

## CI/CD Integration

For CI/CD, use the script:
```yaml
- name: Run E2E Tests
  run: ./run_e2e_tests.sh
```

This ensures all tests pass individually despite GTK threading constraints.
