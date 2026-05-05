//! # Rust Completion Extension — Native Extension
//!
//! A **native extension** (compiled into the binary) that provides Rust-specific
//! code completion data by parsing the locally-installed Rust documentation
//! (via `rustup doc --path`).
//!
//! When enabled and `on_app_start()` fires, it:
//!
//! 1. Parses `std/all.html` and `sidebar-items*.js` from the rustup doc tree.
//! 2. Caches the result to `~/.config/dvop/rust_completions_cache.json`
//!    keyed by toolchain + `rustc` version.
//! 3. Registers the parsed data as the `"rust"` completion provider.
//! 4. If a `completion_data/rust.json` also exists, merges its entries on top.
//!
//! The enable/disable state is persisted in `~/.config/dvop/native_extensions.json`.
//!
//! See FEATURES.md: Feature #111 — Code Completion

use super::native::NativeExtension;
use crate::completion::json_provider::{
    ImportItem, KeywordData, LanguageCompletionData, ModuleData, ModuleHierarchy, SnippetData,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── State ────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    static ref ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

// ── Extension struct ─────────────────────────────────────────────

pub struct RustCompletionExtension;

// "impl" blocks define methods and behavior for a struct or enum.
impl RustCompletionExtension {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn new() -> Self {
        let enabled = load_enabled_state();
        ENABLED.store(enabled, Ordering::SeqCst);
        Self
    }
}

// "impl" blocks define methods and behavior for a struct or enum.
impl NativeExtension for RustCompletionExtension {
    fn id(&self) -> &str {
        "rust-completion"
    }

    fn manifest(&self) -> super::ExtensionManifest {
        super::ExtensionManifest {
            id: "rust-completion".to_string(),
            name: "Rust Completion".to_string(),
            version: "1.0.0".to_string(),
            description: "Rust code completion powered by rustup documentation. Parses std docs at runtime for comprehensive type, function, trait, and macro suggestions.".to_string(),
            author: "Built-in".to_string(),
            enabled: self.is_enabled(),
            icon: None,
            is_native: true,
            contributions: super::ExtensionContributions::default(),
        }
    }

    fn is_enabled(&self) -> bool {
        ENABLED.load(Ordering::SeqCst)
    }

    fn set_enabled(&mut self, enabled: bool) {
        ENABLED.store(enabled, Ordering::SeqCst);
        persist_enabled_state(enabled);
        if enabled {
            load_and_register();
        } else {
            unregister();
        }
    }

    fn on_app_start(&self) {
        if !self.is_enabled() {
            return;
        }
        load_and_register();
    }
}

// ── Public helpers ───────────────────────────────────────────────

/// Register the Rust completion extension. Call once during app init.
pub fn register() {
    // Box::new(...) allocates the data on the heap rather than the stack.
    super::native::register(Box::new(RustCompletionExtension::new()));
}

/// Check if the extension is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

// ── Completion loading ───────────────────────────────────────────

/// Remove Rust completion data registered by this extension.
/// Falls back to `rust.json` if available on disk.
fn unregister() {
    let mut manager = crate::completion::json_provider::get_completion_manager();
    manager.remove_provider("rust");
    // Let the manager re-load rust.json from disk on next access (if it exists)
    println!("Rust completion extension disabled — removed rustup completions");
}

/// Load Rust completions and register them with the completion system.
/// Also merges `rust.json` if it exists.
pub fn load_and_register() {
    if !is_enabled() {
        return;
    }
    let rust_data = load_rust_completions();

    let mut manager = crate::completion::json_provider::get_completion_manager();

    // Check if rust.json was already loaded by initialize_completion_data
    let json_data = manager
        .get_provider("rust")
        .map(|p| p.language_data().clone());

    // Set the rustup data as the base
    manager.add_language_data("rust", rust_data);

    // Merge rust.json entries on top if present
    if let Some(extra) = json_data {
        manager.merge_language_data("rust", extra);
        println!("Merged rust.json data into rustup completions");
    }

    println!("Loaded Rust completions from rustup documentation");
}

/// Load Rust completion data from rustup documentation.
///
/// 1. Checks for a valid disk cache matching the current toolchain.
/// 2. If cache is stale or missing, parses the docs tree from `rustup`.
/// 3. Falls back to hardcoded keywords when `rustup` is unavailable.
pub fn load_rust_completions() -> LanguageCompletionData {
    let toolchain = active_toolchain().unwrap_or_default();
    let version = rust_version().unwrap_or_default();

    // Try cached data first
    if let Some(cached) = load_cache(&toolchain, &version) {
        crate::status_log::log_info(&format!(
            "Rust completions loaded from cache ({})",
            version
        ));
        return cached;
    }

    // Try to parse from rustup docs
    if let Some(docs_root) = rustup_docs_path() {
        crate::status_log::log_info(&format!(
            "Parsing Rust docs from rustup ({})...",
            version
        ));
        let data = parse_docs(&docs_root);
        let count = data.keywords.len();
        // Persist cache
        if !toolchain.is_empty() && !version.is_empty() {
            save_cache(&toolchain, &version, &data);
        }
        crate::status_log::log_success(&format!(
            "Rust completions ready — {} items ({})",
            count, version
        ));
        return data;
    }

    crate::status_log::log_info("rustup not available, using built-in Rust completions");
    fallback_data()
}

// ── Persistence helpers ──────────────────────────────────────────

fn config_path() -> PathBuf {
    if let Some(home) = home::home_dir() {
        home.join(".config").join("dvop").join("native_extensions.json")
    } else {
        PathBuf::from(".config/dvop/native_extensions.json")
    }
}

fn load_enabled_state() -> bool {
    let path = config_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&data) {
            return *map.get("rust-completion").unwrap_or(&true);
        }
    }
    true // enabled by default
}

fn persist_enabled_state(enabled: bool) {
    let path = config_path();
    let mut map: HashMap<String, bool> = if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    };
    map.insert("rust-completion".to_string(), enabled);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Rustup commands ──────────────────────────────────────────────

/// Run `rustup doc --path` and return the doc root directory.
fn rustup_docs_path() -> Option<PathBuf> {
    let output = Command::new("rustup")
        .args(["doc", "--path"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(&path_str);
    path.parent().map(|p| p.to_path_buf())
}

/// Run `rustup show active-toolchain` and return the string.
fn active_toolchain() -> Option<String> {
    let output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tc = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(tc)
}

/// Run `rustc --version` and return the version string (e.g. "rustc 1.91.1").
fn rust_version() -> Option<String> {
    let output = Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let full = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(
        full.split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

// ── Disk cache ───────────────────────────────────────────────────

/// Cache envelope stored on disk.
#[derive(Serialize, Deserialize)]
struct CacheEnvelope {
    toolchain: String,
    rust_version: String,
    data: LanguageCompletionData,
}

fn cache_file_path() -> PathBuf {
    let config_dir = dirs_config().join("dvop");
    config_dir.join("rust_completions_cache.json")
}

fn dirs_config() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from("/tmp")
}

// Option<T> is an enum that represents an optional value: either Some(T) or None.
fn load_cache(toolchain: &str, version: &str) -> Option<LanguageCompletionData> {
    if toolchain.is_empty() || version.is_empty() {
        return None;
    }
    let path = cache_file_path();
    let content = fs::read_to_string(&path).ok()?;
    let envelope: CacheEnvelope = serde_json::from_str(&content).ok()?;
    if envelope.toolchain == toolchain && envelope.rust_version == version {
        Some(envelope.data)
    } else {
        None
    }
}

fn save_cache(toolchain: &str, version: &str, data: &LanguageCompletionData) {
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let envelope = CacheEnvelope {
        toolchain: toolchain.to_string(),
        rust_version: version.to_string(),
        data: data.clone(),
    };
    if let Ok(json) = serde_json::to_string(&envelope) {
        let _ = fs::write(&path, json);
    }
}

// ── Documentation parser ─────────────────────────────────────────

/// Parse the Rust std documentation tree into `LanguageCompletionData`.
fn parse_docs(docs_root: &Path) -> LanguageCompletionData {
    let std_dir = docs_root.join("std");

    let hardcoded_keywords = rust_keywords();
    let snippets = rust_snippets();

    let hardcoded_names: std::collections::HashSet<String> = hardcoded_keywords
        .iter()
        .map(|k| k.keyword.clone())
        .collect();

    let all_html_path = std_dir.join("all.html");
    let all_items = if all_html_path.exists() {
        parse_all_html(&all_html_path)
    } else {
        HashMap::new()
    };

    let mut keywords = Vec::new();

    for (category, items) in &all_items {
        let kw_type = category_to_type(category);
        let kw_category = category_to_keyword_category(category);
        for item in items {
            if hardcoded_names.contains(&item.name) {
                continue;
            }
            keywords.push(KeywordData {
                keyword: item.name.clone(),
                r#type: kw_type.to_string(),
                description: format!(
                    "{} — std::{}",
                    item.name,
                    item.module_path
                ),
                example: format!("use std::{};", item.module_path),
                category: kw_category.to_string(),
            });
        }
    }

    keywords.extend(hardcoded_keywords);

    let imports = parse_module_hierarchy(&std_dir);

    LanguageCompletionData {
        language: "rust".to_string(),
        description: "Rust completion data (from rustup documentation)".to_string(),
        keywords,
        snippets,
        imports: Some(imports),
    }
}

struct AllHtmlItem {
    name: String,
    module_path: String,
}

fn parse_all_html(path: &Path) -> HashMap<String, Vec<AllHtmlItem>> {
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    let html = match fs::read_to_string(path) {
        Ok(h) => h,
        Err(_) => return HashMap::new(),
    };

    let mut result: HashMap<String, Vec<AllHtmlItem>> = HashMap::new();

    let section_re = Regex::new(
        r#"id="(\w+)">[^<]+</h3><ul class="all-items">(.*?)</ul>"#,
    )
    // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
    .unwrap();

    // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
    let item_re = Regex::new(r#"<a href="[^"]+">([^<]+)</a>"#).unwrap();

    for section in section_re.captures_iter(&html) {
        let section_id = section[1].to_string();
        let content = &section[2];

        let mut items = Vec::new();
        for cap in item_re.captures_iter(content) {
            let full_name = cap[1].to_string();
            let name = full_name
                .rsplit("::")
                .next()
                .unwrap_or(&full_name)
                .to_string();
            items.push(AllHtmlItem {
                name,
                module_path: full_name,
            });
        }
        result.insert(section_id, items);
    }

    result
}

fn category_to_type(section_id: &str) -> &'static str {
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    match section_id {
        "structs" => "type",
        "enums" => "type",
        "traits" => "trait",
        "functions" => "function",
        "macros" => "macro",
        "primitives" => "primitive",
        "constants" => "constant",
        "types" => "type",
        "unions" => "type",
        "attributes" => "macro",
        "derives" => "macro",
        _ => "keyword",
    }
}

fn category_to_keyword_category(section_id: &str) -> &'static str {
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    match section_id {
        "structs" => "std_types",
        "enums" => "std_types",
        "traits" => "traits",
        "functions" => "std_functions",
        "macros" => "macros",
        "primitives" => "primitive_types",
        "constants" => "constants",
        "types" => "type_aliases",
        "unions" => "std_types",
        "attributes" => "attributes",
        "derives" => "derives",
        _ => "other",
    }
}

// ── Module hierarchy from sidebar-items ──────────────────────────

fn parse_module_hierarchy(std_dir: &Path) -> ModuleHierarchy {
    let mut modules = Vec::new();
    parse_sidebar_recursive(std_dir, "std", &mut modules, 0);
    ModuleHierarchy { modules }
}

const MAX_DEPTH: usize = 4;

fn parse_sidebar_recursive(
    dir: &Path,
    module_path: &str,
    modules: &mut Vec<ModuleData>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    let sidebar_data = match read_sidebar_items(dir) {
        Some(data) => data,
        None => return,
    };

    let mut items: Vec<ImportItem> = Vec::new();
    let mut submodules: Vec<String> = Vec::new();

    for (item_type, names) in &sidebar_data {
        match item_type.as_str() {
            "mod" => {
                for name in names {
                    submodules.push(name.clone());
                }
            }
            "struct" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "struct".to_string(),
                        description: format!("struct {} in {}", name, module_path),
                    });
                }
            }
            "enum" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "enum".to_string(),
                        description: format!("enum {} in {}", name, module_path),
                    });
                }
            }
            "trait" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "trait".to_string(),
                        description: format!("trait {} in {}", name, module_path),
                    });
                }
            }
            "fn" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "function".to_string(),
                        description: format!("fn {} in {}", name, module_path),
                    });
                }
            }
            "macro" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "macro".to_string(),
                        description: format!("macro {}! in {}", name, module_path),
                    });
                }
            }
            "type" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "type".to_string(),
                        description: format!("type {} in {}", name, module_path),
                    });
                }
            }
            "constant" => {
                for name in names {
                    items.push(ImportItem {
                        name: name.clone(),
                        item_type: "const".to_string(),
                        description: format!("const {} in {}", name, module_path),
                    });
                }
            }
            _ => {}
        }
    }

    modules.push(ModuleData {
        path: module_path.to_string(),
        items,
        submodules: submodules.clone(),
    });

    for submod in &submodules {
        let subdir = dir.join(submod);
        if subdir.is_dir() {
            let child_path = format!("{}::{}", module_path, submod);
            parse_sidebar_recursive(&subdir, &child_path, modules, depth + 1);
        }
    }
}

// Option<T> is an enum that represents an optional value: either Some(T) or None.
fn read_sidebar_items(dir: &Path) -> Option<HashMap<String, Vec<String>>> {
    let entries = fs::read_dir(dir).ok()?;
    let sidebar_file = entries
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_str()
                .map_or(false, |n| n.starts_with("sidebar-items") && n.ends_with(".js"))
        })?;

    let content = fs::read_to_string(sidebar_file.path()).ok()?;

    let json_start = content.find('{')?;
    let json_end = content.rfind('}')?;
    if json_end <= json_start {
        return None;
    }
    let json_str = &content[json_start..=json_end];
    serde_json::from_str(json_str).ok()
}

// ── Hardcoded Rust keywords ──────────────────────────────────────

fn rust_keywords() -> Vec<KeywordData> {
    let kws: &[(&str, &str, &str, &str)] = &[
        ("as", "Performs type casting or renames imports. Converts values between types or creates aliases.", "let x = 65u8 as char;", "keyword"),
        // Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
        ("async", "Marks a function or block as asynchronous, returning a Future instead of blocking.", "async fn fetch() -> Result<String, Error> { Ok(String::new()) }", "async"),
        ("await", "Suspends execution until a Future completes, yielding its result.", "let data = fetch().await?;", "async"),
        ("become", "Reserved for future use (tail calls).", "// reserved keyword", "keyword"),
        ("break", "Exits the current loop immediately, optionally returning a value.", "for x in data { if x == target { break; } }", "control_flow"),
        ("const", "Declares a compile-time constant value that is inlined wherever used.", "const MAX_SIZE: usize = 1024;", "variable_declaration"),
        ("continue", "Skips the rest of the current loop iteration and starts the next one.", "for x in 0..10 { if x % 2 == 0 { continue; } println!(\"{}\", x); }", "control_flow"),
        ("crate", "Refers to the root of the current crate in module paths.", "use crate::utils::helper;", "module_system"),
        ("dyn", "Creates a trait object for dynamic dispatch at runtime.", "fn draw(shape: &dyn Drawable) { shape.draw(); }", "traits"),
        ("else", "Provides an alternative branch when an if condition is false.", "if ready { go(); } else { wait(); }", "control_flow"),
        ("enum", "Defines a type that can be one of several different variants.", "enum Status { Loading, Success(String), Error }", "type_declaration"),
        ("extern", "Links to external code or specifies a foreign ABI.", "extern \"C\" fn callback() {}", "module_system"),
        ("false", "Boolean literal representing the false value.", "let done = false;", "literal_values"),
        ("fn", "Defines a function.", "fn greet(name: &str) { println!(\"Hello, {}!\", name); }", "function_declaration"),
        ("for", "Iterates over collections, ranges, or anything implementing Iterator.", "for item in &my_list { println!(\"{}\", item); }", "control_flow"),
        ("if", "Executes code conditionally based on a boolean expression.", "if temperature > 30 { println!(\"Hot!\"); }", "control_flow"),
        ("impl", "Adds methods and trait implementations to a type.", "impl Person { fn greet(&self) { println!(\"Hi, I'm {}\", self.name); } }", "implementation"),
        ("in", "Used with for loops to specify the iterator.", "for x in 0..10 { println!(\"{}\", x); }", "control_flow"),
        ("let", "Creates a new variable binding. Variables are immutable by default.", "let name = \"Alice\"; let mut count = 0;", "variable_declaration"),
        ("loop", "Creates an infinite loop that runs until explicitly broken.", "loop { if should_stop() { break; } process(); }", "control_flow"),
        ("match", "Compares a value against patterns and executes the matching arm.", "match result { Ok(v) => use_value(v), Err(e) => handle(e) }", "control_flow"),
        // pub makes this function public, allowing it to be used from outside this module.
        ("mod", "Declares a module, grouping related code together.", "mod utils { pub fn helper() {} }", "module_system"),
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        ("move", "Forces a closure to take ownership of captured variables.", "let name = String::from(\"Alice\"); let greet = move || println!(\"{}\", name);", "keyword"),
        ("mut", "Makes a variable binding mutable so its value can be changed.", "let mut score = 0; score += 10;", "variable_declaration"),
        // pub makes this function public, allowing it to be used from outside this module.
        ("pub", "Makes items accessible from outside their module.", "pub fn public_api() {}", "visibility"),
        ("ref", "Binds by reference in pattern matching.", "let ref x = value; // x is &value", "keyword"),
        ("return", "Exits the current function early with a value.", "fn check(x: i32) -> bool { if x > 0 { return true; } false }", "control_flow"),
        ("self", "Refers to the current module or the receiver in methods.", "impl Foo { fn bar(&self) {} }", "module_system"),
        ("Self", "Refers to the implementing type inside impl or trait blocks.", "impl MyType { fn new() -> Self { Self { } } }", "type_declaration"),
        ("static", "Declares a global variable with a fixed memory address for the program's lifetime.", "static COUNTER: AtomicU32 = AtomicU32::new(0);", "variable_declaration"),
        ("struct", "Creates a custom data type by grouping related fields.", "struct Person { name: String, age: u32 }", "type_declaration"),
        ("super", "Refers to the parent module in paths.", "use super::helper;", "module_system"),
        ("trait", "Defines shared behavior that types can implement.", "trait Drawable { fn draw(&self); }", "trait_declaration"),
        ("true", "Boolean literal representing the true value.", "let ready = true;", "literal_values"),
        // Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
        ("type", "Creates a type alias for an existing type.", "type Result<T> = std::result::Result<T, MyError>;", "type_declaration"),
        ("union", "Defines a union type where all fields share the same memory.", "union IntOrFloat { i: i32, f: f32 }", "type_declaration"),
        ("unsafe", "Marks code that bypasses Rust's safety guarantees.", "unsafe { *raw_ptr = 42; }", "keyword"),
        ("use", "Brings items into scope from other modules or crates.", "use std::collections::HashMap;", "module_system"),
        ("where", "Adds constraints to generic type parameters.", "fn process<T>(item: T) where T: Display + Clone {}", "keyword"),
        ("while", "Repeats code as long as a condition is true.", "while !queue.is_empty() { process(queue.pop()); }", "control_flow"),
    ];

    kws.iter()
        .map(|(kw, desc, example, category)| KeywordData {
            keyword: kw.to_string(),
            r#type: "keyword".to_string(),
            description: format!("{} — {}", kw, desc),
            example: example.to_string(),
            category: category.to_string(),
        })
        .collect()
}

// ── Hardcoded snippets ───────────────────────────────────────────

fn rust_snippets() -> Vec<SnippetData> {
    vec![
        SnippetData {
            trigger: "fn".to_string(),
            description: "Function definition".to_string(),
            content: "fn ${1:name}(${2:}) -> ${3:ReturnType} {\n\t${4:todo!()}\n}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "pfn".to_string(),
            description: "Public function definition".to_string(),
            // pub makes this function public, allowing it to be used from outside this module.
            content: "pub fn ${1:name}(${2:}) -> ${3:ReturnType} {\n\t${4:todo!()}\n}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "afn".to_string(),
            description: "Async function definition".to_string(),
            // Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
            content: "async fn ${1:name}(${2:}) -> ${3:Result<()>} {\n\t${4:todo!()}\n}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "struct".to_string(),
            description: "Struct definition with derives".to_string(),
            // #[derive(...)] asks the compiler to automatically generate basic trait implementations.
            content: "#[derive(${1:Debug, Clone})]\nstruct ${2:Name} {\n\t${3:field}: ${4:Type},\n}".to_string(),
            category: "type".to_string(),
        },
        SnippetData {
            trigger: "enum".to_string(),
            description: "Enum definition with derives".to_string(),
            // #[derive(...)] asks the compiler to automatically generate basic trait implementations.
            content: "#[derive(${1:Debug, Clone})]\nenum ${2:Name} {\n\t${3:Variant},\n}".to_string(),
            category: "type".to_string(),
        },
        SnippetData {
            trigger: "impl".to_string(),
            description: "Implementation block".to_string(),
            content: "impl ${1:Type} {\n\t${2:}\n}".to_string(),
            category: "implementation".to_string(),
        },
        SnippetData {
            trigger: "impl_trait".to_string(),
            description: "Trait implementation".to_string(),
            content: "impl ${1:Trait} for ${2:Type} {\n\t${3:}\n}".to_string(),
            category: "implementation".to_string(),
        },
        SnippetData {
            trigger: "trait".to_string(),
            description: "Trait definition".to_string(),
            content: "trait ${1:Name} {\n\t${2:fn method(&self);}\n}".to_string(),
            category: "trait".to_string(),
        },
        SnippetData {
            trigger: "match".to_string(),
            description: "Match expression".to_string(),
            content: "match ${1:expr} {\n\t${2:pattern} => ${3:value},\n\t_ => ${4:default},\n}".to_string(),
            category: "control_flow".to_string(),
        },
        SnippetData {
            trigger: "if_let".to_string(),
            description: "If-let pattern matching".to_string(),
            content: "if let ${1:Some(value)} = ${2:option} {\n\t${3:}\n}".to_string(),
            category: "control_flow".to_string(),
        },
        SnippetData {
            trigger: "while_let".to_string(),
            description: "While-let loop".to_string(),
            content: "while let ${1:Some(item)} = ${2:iter.next()} {\n\t${3:}\n}".to_string(),
            category: "control_flow".to_string(),
        },
        SnippetData {
            trigger: "for_in".to_string(),
            description: "For-in loop".to_string(),
            content: "for ${1:item} in ${2:collection} {\n\t${3:}\n}".to_string(),
            category: "control_flow".to_string(),
        },
        SnippetData {
            trigger: "loop".to_string(),
            description: "Infinite loop".to_string(),
            content: "loop {\n\t${1:}\n\tbreak;\n}".to_string(),
            category: "control_flow".to_string(),
        },
        SnippetData {
            trigger: "test".to_string(),
            description: "Unit test function".to_string(),
            content: "#[test]\nfn ${1:test_name}() {\n\t${2:assert!(true);}\n}".to_string(),
            category: "testing".to_string(),
        },
        SnippetData {
            trigger: "test_mod".to_string(),
            description: "Test module".to_string(),
            content: "#[cfg(test)]\nmod tests {\n\tuse super::*;\n\n\t#[test]\n\tfn ${1:test_name}() {\n\t\t${2:}\n\t}\n}".to_string(),
            category: "testing".to_string(),
        },
        SnippetData {
            trigger: "derive".to_string(),
            description: "Derive attribute".to_string(),
            // #[derive(...)] asks the compiler to automatically generate basic trait implementations.
            content: "#[derive(${1:Debug, Clone, PartialEq})]".to_string(),
            category: "attribute".to_string(),
        },
        SnippetData {
            trigger: "println".to_string(),
            description: "Print line macro".to_string(),
            content: "println!(\"${1:{}}\", ${2:value});".to_string(),
            category: "macro".to_string(),
        },
        SnippetData {
            trigger: "eprintln".to_string(),
            description: "Error print line macro".to_string(),
            content: "eprintln!(\"${1:{}}\", ${2:value});".to_string(),
            category: "macro".to_string(),
        },
        SnippetData {
            trigger: "vec".to_string(),
            description: "Vec macro".to_string(),
            content: "vec![${1:}]".to_string(),
            category: "macro".to_string(),
        },
        SnippetData {
            trigger: "format".to_string(),
            description: "Format string macro".to_string(),
            content: "format!(\"${1:{}}\", ${2:value})".to_string(),
            category: "macro".to_string(),
        },
        SnippetData {
            trigger: "closure".to_string(),
            description: "Closure expression".to_string(),
            content: "|${1:args}| ${2:expr}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "closure_block".to_string(),
            description: "Closure with block body".to_string(),
            content: "|${1:args}| {\n\t${2:}\n}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "result_match".to_string(),
            description: "Match on Result".to_string(),
            content: "match ${1:result} {\n\tOk(${2:value}) => ${3:value},\n\tErr(${4:e}) => ${5:return Err(e.into())},\n}".to_string(),
            category: "error_handling".to_string(),
        },
        SnippetData {
            trigger: "option_match".to_string(),
            description: "Match on Option".to_string(),
            content: "match ${1:option} {\n\tSome(${2:value}) => ${3:value},\n\tNone => ${4:todo!()},\n}".to_string(),
            category: "error_handling".to_string(),
        },
        SnippetData {
            trigger: "new".to_string(),
            description: "Constructor new() method".to_string(),
            content: "pub fn new(${1:}) -> Self {\n\tSelf {\n\t\t${2:}\n\t}\n}".to_string(),
            category: "implementation".to_string(),
        },
        SnippetData {
            trigger: "display".to_string(),
            description: "Display trait implementation".to_string(),
            content: "impl std::fmt::Display for ${1:Type} {\n\tfn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n\t\twrite!(f, \"${2:{}}\", ${3:self})\n\t}\n}".to_string(),
            category: "trait".to_string(),
        },
        SnippetData {
            trigger: "from".to_string(),
            description: "From trait implementation".to_string(),
            content: "impl From<${1:Source}> for ${2:Target} {\n\tfn from(value: ${1:Source}) -> Self {\n\t\t${3:todo!()}\n\t}\n}".to_string(),
            category: "trait".to_string(),
        },
        SnippetData {
            trigger: "default".to_string(),
            description: "Default trait implementation".to_string(),
            content: "impl Default for ${1:Type} {\n\tfn default() -> Self {\n\t\tSelf {\n\t\t\t${2:}\n\t\t}\n\t}\n}".to_string(),
            category: "trait".to_string(),
        },
        SnippetData {
            trigger: "main".to_string(),
            description: "Main function".to_string(),
            content: "fn main() {\n\t${1:}\n}".to_string(),
            category: "function".to_string(),
        },
        SnippetData {
            trigger: "main_result".to_string(),
            description: "Main function with Result".to_string(),
            // Result<T, E> is an enum used for returning and propagating errors: either Ok(T) or Err(E).
            content: "fn main() -> Result<(), Box<dyn std::error::Error>> {\n\t${1:}\n\tOk(())\n}".to_string(),
            category: "function".to_string(),
        },
    ]
}

// ── Fallback data ────────────────────────────────────────────────

/// Minimal completion data when rustup is not available.
fn fallback_data() -> LanguageCompletionData {
    LanguageCompletionData {
        language: "rust".to_string(),
        description: "Rust completion data (fallback — rustup not available)".to_string(),
        keywords: rust_keywords(),
        snippets: rust_snippets(),
        imports: None,
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_keywords_count() {
        let kws = rust_keywords();
        assert!(kws.len() >= 38, "Expected at least 38 keywords, got {}", kws.len());
        let names: Vec<&str> = kws.iter().map(|k| k.keyword.as_str()).collect();
        assert!(names.contains(&"fn"));
        assert!(names.contains(&"let"));
        assert!(names.contains(&"match"));
        assert!(names.contains(&"async"));
        assert!(names.contains(&"unsafe"));
    }

    #[test]
    fn test_rust_snippets_count() {
        let snips = rust_snippets();
        assert!(snips.len() >= 25, "Expected at least 25 snippets, got {}", snips.len());
        let triggers: Vec<&str> = snips.iter().map(|s| s.trigger.as_str()).collect();
        assert!(triggers.contains(&"fn"));
        assert!(triggers.contains(&"struct"));
        assert!(triggers.contains(&"impl"));
        assert!(triggers.contains(&"test"));
    }

    #[test]
    fn test_fallback_data_structure() {
        let data = fallback_data();
        assert_eq!(data.language, "rust");
        assert!(!data.keywords.is_empty());
        assert!(!data.snippets.is_empty());
        assert!(data.imports.is_none());
    }

    #[test]
    fn test_parse_all_html_sample() {
        let sample = r#"<html>
            <h3 id="structs">Structs</h3><ul class="all-items"><li><a href="collections/struct.HashMap.html">collections::HashMap</a></li><li><a href="collections/struct.HashSet.html">collections::HashSet</a></li></ul>
            <h3 id="traits">Traits</h3><ul class="all-items"><li><a href="clone/trait.Clone.html">clone::Clone</a></li></ul>
        </html>"#;

        let tmp = std::env::temp_dir().join("rust_completion_ext_test_all.html");
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        fs::write(&tmp, sample).unwrap();
        let items = parse_all_html(&tmp);
        let _ = fs::remove_file(&tmp);

        assert!(items.contains_key("structs"));
        assert!(items.contains_key("traits"));
        let structs = &items["structs"];
        assert_eq!(structs.len(), 2);
        assert_eq!(structs[0].name, "HashMap");
        assert_eq!(structs[0].module_path, "collections::HashMap");
        assert_eq!(structs[1].name, "HashSet");
        let traits = &items["traits"];
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Clone");
    }

    #[test]
    fn test_read_sidebar_items_sample() {
        let tmp_dir = std::env::temp_dir().join("rust_completion_ext_test_sidebar");
        let _ = fs::create_dir_all(&tmp_dir);
        let sidebar_content =
            r#"window.SIDEBAR_ITEMS = {"struct":["HashMap","HashSet"],"fn":["new"],"mod":["hash_map"]};"#;
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        fs::write(tmp_dir.join("sidebar-items1.91.1.js"), sidebar_content).unwrap();

        let data = read_sidebar_items(&tmp_dir);
        let _ = fs::remove_dir_all(&tmp_dir);

        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data["struct"], vec!["HashMap", "HashSet"]);
        assert_eq!(data["fn"], vec!["new"]);
        assert_eq!(data["mod"], vec!["hash_map"]);
    }

    #[test]
    fn test_category_mappings() {
        assert_eq!(category_to_type("structs"), "type");
        assert_eq!(category_to_type("traits"), "trait");
        assert_eq!(category_to_type("functions"), "function");
        assert_eq!(category_to_type("macros"), "macro");
        assert_eq!(category_to_type("primitives"), "primitive");

        assert_eq!(category_to_keyword_category("structs"), "std_types");
        assert_eq!(category_to_keyword_category("traits"), "traits");
        assert_eq!(category_to_keyword_category("macros"), "macros");
    }

    #[test]
    fn test_load_rust_completions_returns_data() {
        let data = load_rust_completions();
        assert_eq!(data.language, "rust");
        assert!(!data.keywords.is_empty());
        assert!(!data.snippets.is_empty());
        let kw_names: Vec<&str> = data.keywords.iter().map(|k| k.keyword.as_str()).collect();
        assert!(kw_names.contains(&"fn"));
        assert!(kw_names.contains(&"let"));
    }

    #[test]
    fn test_cache_roundtrip() {
        let data = fallback_data();
        let toolchain = "test-toolchain-for-unit-test";
        let version = "rustc 99.0.0";
        save_cache(toolchain, version, &data);
        let loaded = load_cache(toolchain, version);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.language, "rust");
        assert_eq!(loaded.keywords.len(), data.keywords.len());
        assert!(load_cache(toolchain, "rustc 99.1.0").is_none());
        let _ = fs::remove_file(cache_file_path());
    }
}
