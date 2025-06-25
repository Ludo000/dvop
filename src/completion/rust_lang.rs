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
            // Basic language constructs
            ("fn", "fn ${1:function_name}(${2:parameters}) -> ${3:return_type} {\n    ${4:// body}\n}"),
            ("struct", "struct ${1:Name} {\n    ${2:field}: ${3:Type},\n}"),
            ("impl", "impl ${1:Type} {\n    ${2:// methods}\n}"),
            ("match", "match ${1:expression} {\n    ${2:pattern} => ${3:result},\n}"),
            ("if", "if ${1:condition} {\n    ${2:// body}\n}"),
            ("iflet", "if let ${1:Some(value)} = ${2:expression} {\n    ${3:// body}\n}"),
            ("for", "for ${1:item} in ${2:iterator} {\n    ${3:// body}\n}"),
            ("while", "while ${1:condition} {\n    ${2:// body}\n}"),
            ("loop", "loop {\n    ${1:// body}\n    break;\n}"),
            
            // Functions and main
            ("main", "fn main() {\n    ${1:// code}\n}"),
            ("mainargs", "fn main() -> Result<(), Box<dyn std::error::Error>> {\n    ${1:// code}\n    Ok(())\n}"),
            ("async_main", "#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    ${1:// async code}\n    Ok(())\n}"),
            ("pub_fn", "pub fn ${1:function_name}(${2:parameters}) -> ${3:return_type} {\n    ${4:// body}\n}"),
            ("async_fn", "async fn ${1:function_name}(${2:parameters}) -> ${3:return_type} {\n    ${4:// async body}\n}"),
            
            // Error handling
            ("result", "Result<${1:T}, ${2:E}>"),
            ("option", "Option<${1:T}>"),
            ("unwrap_or", "${1:expression}.unwrap_or(${2:default})"),
            ("unwrap_or_else", "${1:expression}.unwrap_or_else(|| ${2:default})"),
            ("map_err", "${1:expression}.map_err(|e| ${2:transform})?"),
            ("try_block", "{\n    let result = ${1:operation}?;\n    ${2:// continue}\n    Ok(result)\n}"),
            ("error_match", "match ${1:result} {\n    Ok(${2:value}) => ${3:// handle success},\n    Err(${4:error}) => ${5:// handle error},\n}"),
            
            // Collections and iterators
            ("vec_new", "let ${1:vec} = Vec::new();"),
            ("vec_with", "let ${1:vec} = vec![${2:1, 2, 3}];"),
            ("hashmap", "use std::collections::HashMap;\nlet mut ${1:map} = HashMap::new();"),
            ("hashset", "use std::collections::HashSet;\nlet mut ${1:set} = HashSet::new();"),
            ("collect", "${1:iterator}.collect::<${2:Vec<_>}>()"),
            ("filter_map", "${1:iterator}.filter_map(|${2:item}| ${3:// transform})"),
            ("fold", "${1:iterator}.fold(${2:initial}, |${3:acc}, ${4:item}| ${5:// combine})"),
            ("enumerate", "for (${1:index}, ${2:item}) in ${3:iterator}.enumerate() {\n    ${4:// body}\n}"),
            
            // Async/await patterns
            ("async_block", "async {\n    ${1:// async code}\n}.await"),
            ("spawn", "tokio::spawn(async move {\n    ${1:// async task}\n})"),
            ("join", "tokio::join!(${1:future1}, ${2:future2})"),
            ("select", "tokio::select! {\n    ${1:result1} = ${2:future1} => {\n        ${3:// handle result1}\n    },\n    ${4:result2} = ${5:future2} => {\n        ${6:// handle result2}\n    },\n}"),
            ("timeout", "tokio::time::timeout(Duration::from_secs(${1:5}), ${2:future}).await"),
            
            // Testing
            ("test", "#[test]\nfn ${1:test_name}() {\n    ${2:// test code}\n}"),
            ("test_async", "#[tokio::test]\nasync fn ${1:test_name}() {\n    ${2:// async test code}\n}"),
            ("assert_eq", "assert_eq!(${1:expected}, ${2:actual});"),
            ("assert_ne", "assert_ne!(${1:not_expected}, ${2:actual});"),
            ("assert", "assert!(${1:condition});"),
            ("panic", "panic!(\"${1:message}\");"),
            ("should_panic", "#[test]\n#[should_panic]\nfn ${1:test_name}() {\n    ${2:// code that should panic}\n}"),
            
            // Structs and traits
            ("derive", "#[derive(${1:Debug, Clone})]\nstruct ${2:Name} {\n    ${3:field}: ${4:Type},\n}"),
            ("enum", "enum ${1:Name} {\n    ${2:Variant1},\n    ${3:Variant2}(${4:Type}),\n}"),
            ("trait", "trait ${1:TraitName} {\n    ${2:// trait methods}\n}"),
            ("impl_trait", "impl ${1:Trait} for ${2:Type} {\n    ${3:// implement methods}\n}"),
            ("impl_default", "impl Default for ${1:Type} {\n    fn default() -> Self {\n        Self {\n            ${2:// default values}\n        }\n    }\n}"),
            ("impl_display", "impl std::fmt::Display for ${1:Type} {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        write!(f, \"${2:format}\")\n    }\n}"),
            
            // Memory management
            ("box", "Box::new(${1:value})"),
            ("rc", "Rc::new(${1:value})"),
            ("arc", "Arc::new(${1:value})"),
            ("refcell", "RefCell::new(${1:value})"),
            ("mutex", "Mutex::new(${1:value})"),
            ("rwlock", "RwLock::new(${1:value})"),
            ("weak", "Arc::downgrade(&${1:arc})"),
            
            // Closures and functional programming
            ("closure", "|${1:params}| ${2:body}"),
            ("move_closure", "move |${1:params}| ${2:body}"),
            ("map", "${1:iterator}.map(|${2:item}| ${3:transform})"),
            ("filter", "${1:iterator}.filter(|${2:item}| ${3:condition})"),
            ("find", "${1:iterator}.find(|${2:item}| ${3:condition})"),
            ("any", "${1:iterator}.any(|${2:item}| ${3:condition})"),
            ("all", "${1:iterator}.all(|${2:item}| ${3:condition})"),
            
            // Modules and imports
            ("mod", "mod ${1:module_name} {\n    ${2:// module contents}\n}"),
            ("use", "use ${1:crate}::${2:module}::${3:item};"),
            ("use_std", "use std::${1:module}::${2:item};"),
            ("extern_crate", "extern crate ${1:crate_name};"),
            ("pub_mod", "pub mod ${1:module_name};"),
            ("pub_use", "pub use ${1:path};"),
            
            // Macros
            ("macro_rules", "macro_rules! ${1:macro_name} {\n    (${2:pattern}) => {\n        ${3:expansion}\n    };\n}"),
            ("println", "println!(\"${1:message}\");"),
            ("eprintln", "eprintln!(\"${1:error message}\");"),
            ("format", "format!(\"${1:format string}\", ${2:args})"),
            ("dbg", "dbg!(${1:expression})"),
            ("todo", "todo!(\"${1:implement this}\")"),
            ("unimplemented", "unimplemented!(\"${1:not yet implemented}\")"),
            
            // File I/O and serialization
            ("read_file", "use std::fs;\nlet ${1:contents} = fs::read_to_string(\"${2:path}\")?;"),
            ("write_file", "use std::fs;\nfs::write(\"${1:path}\", ${2:contents})?;"),
            ("open_file", "use std::fs::File;\nlet ${1:file} = File::open(\"${2:path}\")?;"),
            ("create_file", "use std::fs::File;\nlet ${1:file} = File::create(\"${2:path}\")?;"),
            ("serde_derive", "#[derive(Serialize, Deserialize)]\nstruct ${1:Name} {\n    ${2:field}: ${3:Type},\n}"),
            
            // Network and HTTP
            ("http_get", "let ${1:response} = reqwest::get(\"${2:url}\").await?;"),
            ("http_post", "let ${1:response} = reqwest::Client::new()\n    .post(\"${2:url}\")\n    .json(&${3:body})\n    .send()\n    .await?;"),
            
            // Common patterns
            ("builder", "pub struct ${1:Builder} {\n    ${2:field}: Option<${3:Type}>,\n}\n\nimpl ${1:Builder} {\n    pub fn new() -> Self {\n        Self { ${2:field}: None }\n    }\n    \n    pub fn ${2:field}(mut self, ${2:field}: ${3:Type}) -> Self {\n        self.${2:field} = Some(${2:field});\n        self\n    }\n    \n    pub fn build(self) -> Result<${4:Target}, &'static str> {\n        Ok(${4:Target} {\n            ${2:field}: self.${2:field}.ok_or(\"${2:field} is required\")?,\n        })\n    }\n}"),
            ("singleton", "use std::sync::Once;\nstatic INIT: Once = Once::new();\nstatic mut INSTANCE: Option<${1:Type}> = None;\n\nimpl ${1:Type} {\n    pub fn instance() -> &'static ${1:Type} {\n        unsafe {\n            INIT.call_once(|| {\n                INSTANCE = Some(${1:Type}::new());\n            });\n            INSTANCE.as_ref().unwrap()\n        }\n    }\n}"),
            ("new_type", "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ${1:NewType}(${2:InnerType});\n\nimpl ${1:NewType} {\n    pub fn new(value: ${2:InnerType}) -> Self {\n        Self(value)\n    }\n    \n    pub fn into_inner(self) -> ${2:InnerType} {\n        self.0\n    }\n}"),
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
