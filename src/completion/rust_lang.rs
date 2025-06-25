// Rust language completion provider

use super::LanguageProvider;

pub struct RustProvider;

impl LanguageProvider for RustProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
            "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
            "String", "Vec", "HashMap", "HashSet", "Option", "Result", "println!", "eprintln!", "format!",
            "vec!", "Some", "None", "Ok", "Err", "Clone", "Copy", "Debug", "Default", "PartialEq",
            "Eq", "PartialOrd", "Ord", "Hash", "Display", "From", "Into", "TryFrom", "TryInto",
            "Box", "Arc", "Rc", "RefCell", "Cell", "Mutex", "RwLock", "thread", "spawn", "join"
        ]
    }

    fn snippets(&self) -> &[(&'static str, &'static str)] {
        &[
            ("fn", "fn ${1:function_name}(${2:parameters}) -> ${3:return_type} {\n    ${4:// body}\n}"),
            ("struct", "struct ${1:Name} {\n    ${2:field}: ${3:Type},\n}"),
            ("impl", "impl ${1:Type} {\n    ${2:// methods}\n}"),
            ("match", "match ${1:expression} {\n    ${2:pattern} => ${3:result},\n}"),
            ("if", "if ${1:condition} {\n    ${2:// body}\n}"),
            ("for", "for ${1:item} in ${2:iterator} {\n    ${3:// body}\n}"),
            ("while", "while ${1:condition} {\n    ${2:// body}\n}"),
            ("loop", "loop {\n    ${1:// body}\n    break;\n}"),
            ("main", "fn main() {\n    ${1:// code}\n}"),
            ("test", "#[test]\nfn ${1:test_name}() {\n    ${2:// test code}\n}"),
            ("derive", "#[derive(${1:Debug, Clone})]\nstruct ${2:Name} {\n    ${3:field}: ${4:Type},\n}"),
            ("enum", "enum ${1:Name} {\n    ${2:Variant1},\n    ${3:Variant2}(${4:Type}),\n}"),
            ("mod", "mod ${1:module_name} {\n    ${2:// module contents}\n}"),
            ("use", "use ${1:crate}::${2:module}::${3:item};"),
            ("trait", "trait ${1:TraitName} {\n    ${2:// trait methods}\n}"),
        ]
    }

    fn get_documentation(&self, keyword: &str) -> String {
        match keyword {
            "fn" => "fn keyword - Define a function\n\nSyntax: fn name(params) -> return_type { body }".to_string(),
            "struct" => "struct keyword - Define a structure\n\nSyntax: struct Name { field: Type }".to_string(),
            "enum" => "enum keyword - Define an enumeration\n\nSyntax: enum Name { Variant1, Variant2 }".to_string(),
            "impl" => "impl keyword - Implement methods for a type\n\nSyntax: impl Type { fn method(&self) {} }".to_string(),
            "match" => "match keyword - Pattern matching\n\nSyntax: match expr { pattern => result }".to_string(),
            "Vec" => "Vec<T> - A growable array type\n\nExample: let v = Vec::new(); v.push(1);".to_string(),
            "Option" => "Option<T> - Represents optional values\n\nVariants: Some(T), None".to_string(),
            "Result" => "Result<T, E> - Error handling type\n\nVariants: Ok(T), Err(E)".to_string(),
            "let" => "let keyword - Variable binding\n\nSyntax: let variable = value; or let mut variable = value;".to_string(),
            "mut" => "mut keyword - Mutable variable modifier\n\nUsage: let mut x = 5; x = 10;".to_string(),
            "trait" => "trait keyword - Define a trait (interface)\n\nSyntax: trait Name { fn method(&self); }".to_string(),
            "use" => "use keyword - Import items into scope\n\nSyntax: use crate::module::item;".to_string(),
            "mod" => "mod keyword - Define a module\n\nSyntax: mod name { /* contents */ }".to_string(),
            "async" => "async keyword - Define asynchronous function\n\nSyntax: async fn name() { await expr; }".to_string(),
            "await" => "await keyword - Wait for async operation\n\nSyntax: let result = async_fn().await;".to_string(),
            "Box" => "Box<T> - Heap-allocated smart pointer\n\nUsage: let boxed = Box::new(value);".to_string(),
            "Arc" => "Arc<T> - Atomic reference counter for shared ownership\n\nUsage: let shared = Arc::new(value);".to_string(),
            "Rc" => "Rc<T> - Reference counter for shared ownership (single-threaded)\n\nUsage: let shared = Rc::new(value);".to_string(),
            "RefCell" => "RefCell<T> - Interior mutability with runtime borrow checking\n\nUsage: let cell = RefCell::new(value);".to_string(),
            "Mutex" => "Mutex<T> - Mutual exclusion primitive for thread safety\n\nUsage: let mutex = Mutex::new(value);".to_string(),
            "RwLock" => "RwLock<T> - Reader-writer lock for concurrent access\n\nUsage: let lock = RwLock::new(value);".to_string(),
            "println!" => "println! macro - Print to stdout with newline\n\nSyntax: println!(\"format {}\", args);".to_string(),
            "format!" => "format! macro - Create formatted string\n\nSyntax: let s = format!(\"format {}\", args);".to_string(),
            "vec!" => "vec! macro - Create vector with initial values\n\nSyntax: let v = vec![1, 2, 3];".to_string(),
            _ => format!("{} - Rust keyword/identifier", keyword),
        }
    }
}
