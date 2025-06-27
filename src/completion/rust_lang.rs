// Rust language completion provider

use super::LanguageProvider;

pub struct RustProvider;

impl LanguageProvider for RustProvider {
    fn keywords(&self) -> &[&'static str] {
        &[
            // Core language keywords
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while",
            
            // Reserved keywords (future use)
            "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", 
            "unsized", "virtual", "yield", "try", "union",
            
            // Common types
            "String", "str", "Vec", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "LinkedList",
            "VecDeque", "BinaryHeap", "Option", "Result", "Box", "Arc", "Rc", "RefCell", "Cell",
            "Mutex", "RwLock", "Weak", "Pin", "ManuallyDrop", "MaybeUninit",
            
            // Numeric types
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char",
            
            // Option and Result variants
            "Some", "None", "Ok", "Err",
            
            // Common traits
            "Clone", "Copy", "Debug", "Default", "PartialEq", "Eq", "PartialOrd", "Ord", "Hash",
            "Display", "From", "Into", "TryFrom", "TryInto", "AsRef", "AsMut", "Borrow", "BorrowMut",
            "Deref", "DerefMut", "Drop", "Send", "Sync", "Sized", "Unpin", "Future", "Iterator",
            "IntoIterator", "DoubleEndedIterator", "ExactSizeIterator", "Extend", "FromIterator",
            "Fn", "FnMut", "FnOnce", "ToOwned", "ToString", "Error", "Read", "Write", "Seek",
            "BufRead", "BufWriter", "Serialize", "Deserialize",
            
            // Standard library modules and types
            "std", "core", "alloc", "collections", "fs", "io", "net", "path", "sync", "thread",
            "time", "mem", "ptr", "slice", "fmt", "str", "string", "vec", "ops", "cmp", "convert",
            "iter", "marker", "any", "borrow", "boxed", "cell", "rc", "env", "process", "ffi",
            "os", "hash", "num", "rand", "regex", "serde", "tokio", "futures", "async_std",
            
            // File and Path types
            "Path", "PathBuf", "File", "OpenOptions", "DirEntry", "ReadDir", "Metadata", "Permissions",
            
            // IO types
            "Read", "Write", "BufRead", "BufReader", "BufWriter", "Cursor", "Stdin", "Stdout", "Stderr",
            "Lines", "Split", "Bytes", "Chars", "Take", "Chain", "Repeat",
            
            // Threading and concurrency
            "Thread", "ThreadId", "JoinHandle", "Builder", "LocalKey", "Barrier", "Condvar",
            "Once", "OnceWith", "Atomic", "AtomicBool", "AtomicI8", "AtomicI16", "AtomicI32", 
            "AtomicI64", "AtomicIsize", "AtomicU8", "AtomicU16", "AtomicU32", "AtomicU64", "AtomicUsize",
            "AtomicPtr", "Ordering", "Relaxed", "Acquire", "Release", "AcqRel", "SeqCst",
            
            // Time types
            "Duration", "Instant", "SystemTime", "UNIX_EPOCH",
            
            // Network types
            "TcpListener", "TcpStream", "UdpSocket", "IpAddr", "Ipv4Addr", "Ipv6Addr", "SocketAddr",
            "SocketAddrV4", "SocketAddrV6", "ToSocketAddrs",
            
            // Process types
            "Command", "Child", "ChildStdin", "ChildStdout", "ChildStderr", "ExitStatus", "Output",
            "Stdio",
            
            // Memory management
            "Layout", "GlobalAlloc", "System", "handle_alloc_error", "set_alloc_error_hook",
            
            // Macros
            "println!", "eprintln!", "print!", "eprint!", "format!", "format_args!", "vec!",
            "panic!", "assert!", "assert_eq!", "assert_ne!", "debug_assert!", "debug_assert_eq!",
            "debug_assert_ne!", "unreachable!", "unimplemented!", "todo!", "compile_error!",
            "concat!", "stringify!", "include!", "include_str!", "include_bytes!", "env!",
            "option_env!", "cfg!", "line!", "column!", "file!", "module_path!", "matches!",
            "dbg!", "write!", "writeln!", "try!", "macro_rules!", "cfg_attr!", "derive!",
            
            // Attributes
            "allow", "warn", "deny", "forbid", "deprecated", "must_use", "repr", "derive",
            "cfg", "cfg_attr", "test", "bench", "should_panic", "ignore", "inline", "cold",
            "target_feature", "no_mangle", "export_name", "link_name", "link", "used", "crate_type",
            "no_main", "no_std", "recursion_limit", "type_length_limit", "feature", "macro_use",
            "macro_export", "proc_macro", "proc_macro_derive", "proc_macro_attribute", "global_allocator",
            "panic_handler", "alloc_error_handler", "lang", "start", "main", "windows_subsystem",
            
            // Common derive traits
            "Hash", "PartialEq", "Eq", "PartialOrd", "Ord", "Clone", "Copy", "Debug", "Default",
            
            // Error handling
            "Error", "ErrorKind", "Result", "Ok", "Err", "unwrap", "expect", "unwrap_or",
            "unwrap_or_else", "unwrap_or_default", "map", "map_err", "and_then", "or_else",
            "is_ok", "is_err", "as_ref", "as_mut",
            
            // Option methods
            "is_some", "is_none", "contains", "map", "map_or", "map_or_else", "ok_or", "ok_or_else",
            "and", "and_then", "filter", "or", "or_else", "xor", "get_or_insert", "get_or_insert_with",
            "take", "replace", "zip", "unzip",
            
            // Iterator methods
            "collect", "fold", "reduce", "for_each", "filter", "map", "enumerate", "zip", "chain",
            "take", "take_while", "skip", "skip_while", "step_by", "rev", "cycle", "find", "find_map",
            "position", "rposition", "any", "all", "count", "last", "nth", "max", "min", "max_by",
            "min_by", "max_by_key", "min_by_key", "sum", "product", "cmp", "partial_cmp", "flatten",
            "flat_map", "inspect", "by_ref", "partition", "try_fold", "try_for_each", "cloned",
            "copied", "peekable", "fuse",
            
            // Vec methods
            "new", "with_capacity", "push", "pop", "insert", "remove", "clear", "len", "is_empty",
            "capacity", "reserve", "reserve_exact", "shrink_to_fit", "truncate", "as_slice",
            "as_mut_slice", "swap_remove", "drain", "retain", "append", "split_off", "resize",
            "resize_with", "extend_from_slice", "dedup", "dedup_by", "dedup_by_key", "sort",
            "sort_by", "sort_by_key", "sort_unstable", "sort_unstable_by", "sort_unstable_by_key",
            "binary_search", "binary_search_by", "binary_search_by_key", "contains", "starts_with",
            "ends_with", "first", "last", "get", "get_mut", "swap", "reverse", "splice", "split_at",
            "split_at_mut",
            
            // String methods
            "len", "is_empty", "push", "push_str", "pop", "remove", "insert", "insert_str", "clear",
            "truncate", "drain", "replace_range", "chars", "char_indices", "bytes", "split",
            "split_whitespace", "lines", "contains", "starts_with", "ends_with", "find", "rfind",
            "matches", "rmatches", "trim", "trim_start", "trim_end", "trim_matches", "trim_start_matches",
            "trim_end_matches", "parse", "repeat", "to_lowercase", "to_uppercase", "to_ascii_lowercase",
            "to_ascii_uppercase", "escape_debug", "escape_default", "escape_unicode", "replacen",
            "replace", "split_once", "rsplit_once", "strip_prefix", "strip_suffix", "is_ascii",
            
            // Common constants
            "MAX", "MIN", "INFINITY", "NEG_INFINITY", "NAN", "EPSILON", "MANTISSA_DIGITS", "DIGITS",
            "RADIX", "MAX_10_EXP", "MAX_EXP", "MIN_10_EXP", "MIN_EXP", "MIN_POSITIVE",
            
            // Async/await keywords and types
            "async", "await", "Future", "Poll", "Ready", "Pending", "Pin", "Unpin", "Context",
            "Waker", "Wake", "task", "executor", "spawn", "block_on", "yield_now", "sleep",
            "timeout", "interval", "join", "try_join", "select", "race", "FutureExt", "StreamExt",
            "SinkExt", "Stream", "Sink", "AsyncRead", "AsyncWrite", "AsyncSeek", "AsyncBufRead",
            
            // Testing keywords
            "test", "bench", "should_panic", "ignore", "cfg", "feature", "TestResult", "Bencher",
            
            // FFI keywords
            "extern", "c_void", "c_char", "c_int", "c_uint", "c_long", "c_ulong", "c_float", "c_double",
            "CStr", "CString", "OsStr", "OsString", "ffi",
            
            // Proc macro keywords
            "proc_macro", "TokenStream", "TokenTree", "Span", "Delimiter", "Group", "Ident", "Punct",
            "Literal", "quote", "syn", "parse", "parse_macro_input", "DeriveInput", "Data", "Fields",
            "Variant", "Attribute", "Meta", "NestedMeta", "Lit", "Type", "Expr", "Stmt", "Item",
            
            // Unsafe operations
            "unsafe", "raw", "NonNull", "null", "null_mut", "dangling", "drop_in_place", "forget",
            "transmute", "transmute_copy", "size_of", "size_of_val", "align_of", "align_of_val",
            "needs_drop", "discriminant", "Discriminant", "ManuallyDrop", "MaybeUninit", "assume_init",
            "zeroed", "uninitialized", "replace", "swap", "take",
            
            // SIMD types (if std_detect is available)
            "Simd", "LaneCount", "SupportedLaneCount", "SimdElement", "SimdPartialEq", "SimdPartialOrd",
            
            // Common third-party crate keywords
            "serde", "tokio", "reqwest", "clap", "anyhow", "thiserror", "log", "env_logger", "tracing",
            "chrono", "uuid", "regex", "rayon", "crossbeam", "dashmap", "once_cell", "lazy_static",
            "parking_lot", "flume", "criterion", "proptest", "quickcheck", "mockall", "wiremock",
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
            
            // Advanced patterns and modern Rust
            ("state_machine", "enum ${1:State} {\n    ${2:Initial},\n    ${3:Processing},\n    ${4:Complete},\n}\n\nstruct ${5:StateMachine} {\n    state: ${1:State},\n}\n\nimpl ${5:StateMachine} {\n    pub fn new() -> Self {\n        Self { state: ${1:State}::${2:Initial} }\n    }\n    \n    pub fn transition(&mut self) -> Result<(), &'static str> {\n        self.state = match self.state {\n            ${1:State}::${2:Initial} => ${1:State}::${3:Processing},\n            ${1:State}::${3:Processing} => ${1:State}::${4:Complete},\n            ${1:State}::${4:Complete} => return Err(\"Already complete\"),\n        };\n        Ok(())\n    }\n}"),
            
            ("observer", "use std::collections::HashMap;\nuse std::sync::{Arc, Mutex};\n\npub trait Observer {\n    fn update(&self, event: &str);\n}\n\npub struct ${1:Subject} {\n    observers: Arc<Mutex<Vec<Box<dyn Observer + Send + Sync>>>>,\n}\n\nimpl ${1:Subject} {\n    pub fn new() -> Self {\n        Self {\n            observers: Arc::new(Mutex::new(Vec::new())),\n        }\n    }\n    \n    pub fn attach(&self, observer: Box<dyn Observer + Send + Sync>) {\n        self.observers.lock().unwrap().push(observer);\n    }\n    \n    pub fn notify(&self, event: &str) {\n        for observer in self.observers.lock().unwrap().iter() {\n            observer.update(event);\n        }\n    }\n}"),
            
            ("command", "pub trait Command {\n    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;\n    fn undo(&self) -> Result<(), Box<dyn std::error::Error>>;\n}\n\npub struct ${1:ConcreteCommand} {\n    ${2:data}: ${3:String},\n}\n\nimpl Command for ${1:ConcreteCommand} {\n    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {\n        println!(\"Executing: {}\", self.${2:data});\n        Ok(())\n    }\n    \n    fn undo(&self) -> Result<(), Box<dyn std::error::Error>> {\n        println!(\"Undoing: {}\", self.${2:data});\n        Ok(())\n    }\n}\n\npub struct Invoker {\n    commands: Vec<Box<dyn Command>>,\n}\n\nimpl Invoker {\n    pub fn new() -> Self {\n        Self { commands: Vec::new() }\n    }\n    \n    pub fn execute_command(&mut self, command: Box<dyn Command>) -> Result<(), Box<dyn std::error::Error>> {\n        command.execute()?;\n        self.commands.push(command);\n        Ok(())\n    }\n}"),
            
            // Generic and lifetime patterns
            ("generic_struct", "struct ${1:Container}<T> {\n    value: T,\n}\n\nimpl<T> ${1:Container}<T> {\n    pub fn new(value: T) -> Self {\n        Self { value }\n    }\n    \n    pub fn get(&self) -> &T {\n        &self.value\n    }\n    \n    pub fn set(&mut self, value: T) {\n        self.value = value;\n    }\n}"),
            
            ("lifetime_struct", "struct ${1:Reference}<'a> {\n    data: &'a ${2:str},\n}\n\nimpl<'a> ${1:Reference}<'a> {\n    pub fn new(data: &'a ${2:str}) -> Self {\n        Self { data }\n    }\n    \n    pub fn get_data(&self) -> &${2:str} {\n        self.data\n    }\n}"),
            
            ("where_clause", "fn ${1:function_name}<T>() -> T\nwhere\n    T: ${2:Clone} + ${3:Debug} + ${4:Default},\n{\n    ${5:// implementation}\n    T::default()\n}"),
            
            ("associated_types", "trait ${1:Iterator} {\n    type Item;\n    \n    fn next(&mut self) -> Option<Self::Item>;\n}\n\nstruct ${2:Counter} {\n    current: usize,\n    max: usize,\n}\n\nimpl ${1:Iterator} for ${2:Counter} {\n    type Item = usize;\n    \n    fn next(&mut self) -> Option<Self::Item> {\n        if self.current < self.max {\n            let current = self.current;\n            self.current += 1;\n            Some(current)\n        } else {\n            None\n        }\n    }\n}"),
            
            // Async and concurrency patterns
            ("channel", "use tokio::sync::mpsc;\n\n// Create channel\nlet (tx, mut rx) = mpsc::channel::<${1:MessageType}>(${2:32});\n\n// Sender task\nlet sender_handle = tokio::spawn(async move {\n    for i in 0..10 {\n        if tx.send(${3:message}).await.is_err() {\n            break;\n        }\n    }\n});\n\n// Receiver task\nlet receiver_handle = tokio::spawn(async move {\n    while let Some(${4:msg}) = rx.recv().await {\n        println!(\"Received: {:?}\", ${4:msg});\n    }\n});\n\n// Wait for completion\ntokio::try_join!(sender_handle, receiver_handle)?;"),
            
            ("actor", "use tokio::sync::mpsc;\n\npub enum ${1:Message} {\n    ${2:GetValue} { respond_to: oneshot::Sender<${3:i32}> },\n    ${4:SetValue} { value: ${3:i32} },\n}\n\npub struct ${5:Actor} {\n    receiver: mpsc::Receiver<${1:Message}>,\n    value: ${3:i32},\n}\n\nimpl ${5:Actor} {\n    pub fn new(receiver: mpsc::Receiver<${1:Message}>) -> Self {\n        Self { receiver, value: 0 }\n    }\n    \n    pub async fn run(mut self) {\n        while let Some(msg) = self.receiver.recv().await {\n            match msg {\n                ${1:Message}::${2:GetValue} { respond_to } => {\n                    let _ = respond_to.send(self.value);\n                }\n                ${1:Message}::${4:SetValue} { value } => {\n                    self.value = value;\n                }\n            }\n        }\n    }\n}\n\n#[derive(Clone)]\npub struct ${6:ActorHandle} {\n    sender: mpsc::Sender<${1:Message}>,\n}\n\nimpl ${6:ActorHandle} {\n    pub fn new() -> Self {\n        let (sender, receiver) = mpsc::channel(8);\n        let actor = ${5:Actor}::new(receiver);\n        tokio::spawn(actor.run());\n        Self { sender }\n    }\n    \n    pub async fn get_value(&self) -> Result<${3:i32}, Box<dyn std::error::Error>> {\n        let (send, recv) = oneshot::channel();\n        self.sender.send(${1:Message}::${2:GetValue} { respond_to: send }).await?;\n        Ok(recv.await?)\n    }\n    \n    pub async fn set_value(&self, value: ${3:i32}) -> Result<(), Box<dyn std::error::Error>> {\n        self.sender.send(${1:Message}::${4:SetValue} { value }).await?;\n        Ok(())\n    }\n}"),
            
            ("stream", "use futures::stream::{self, StreamExt};\nuse tokio_stream::wrappers::IntervalStream;\n\n// Create a stream from values\nlet stream = stream::iter(vec![${1:1, 2, 3, 4, 5}]);\n\n// Process stream\nlet result: Vec<${2:i32}> = stream\n    .map(|x| x * 2)\n    .filter(|&x| x > ${3:5})\n    .collect()\n    .await;\n\n// Interval stream\nlet interval = tokio::time::interval(std::time::Duration::from_secs(1));\nlet interval_stream = IntervalStream::new(interval);\n\ninterval_stream\n    .take(${4:5})\n    .for_each(|_| async {\n        println!(\"Tick!\");\n    })\n    .await;"),
            
            // Error handling patterns
            ("custom_error", "use std::fmt;\n\n#[derive(Debug)]\npub enum ${1:MyError} {\n    ${2:InvalidInput}(String),\n    ${3:NetworkError},\n    ${4:ParseError}(std::num::ParseIntError),\n}\n\nimpl fmt::Display for ${1:MyError} {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {\n            ${1:MyError}::${2:InvalidInput}(msg) => write!(f, \"Invalid input: {}\", msg),\n            ${1:MyError}::${3:NetworkError} => write!(f, \"Network error occurred\"),\n            ${1:MyError}::${4:ParseError}(err) => write!(f, \"Parse error: {}\", err),\n        }\n    }\n}\n\nimpl std::error::Error for ${1:MyError} {\n    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {\n        match self {\n            ${1:MyError}::${4:ParseError}(err) => Some(err),\n            _ => None,\n        }\n    }\n}\n\nimpl From<std::num::ParseIntError> for ${1:MyError} {\n    fn from(err: std::num::ParseIntError) -> Self {\n        ${1:MyError}::${4:ParseError}(err)\n    }\n}"),
            
            ("anyhow_error", "use anyhow::{Context, Result};\n\nfn ${1:function_name}() -> Result<${2:ReturnType}> {\n    let result = ${3:operation}()\n        .context(\"Failed to ${4:describe_operation}\")?;\n    \n    Ok(result)\n}"),
            
            ("thiserror", "use thiserror::Error;\n\n#[derive(Error, Debug)]\npub enum ${1:MyError} {\n    #[error(\"Invalid input: {message}\")]\n    ${2:InvalidInput} { message: String },\n    \n    #[error(\"IO error\")]\n    ${3:Io}(#[from] std::io::Error),\n    \n    #[error(\"Parse error\")]\n    ${4:Parse}(#[from] std::num::ParseIntError),\n    \n    #[error(\"Custom error: {0}\")]\n    ${5:Custom}(String),\n}"),
            
            // Web and API patterns
            ("axum_handler", "use axum::{\n    extract::{Path, Query, State},\n    http::StatusCode,\n    response::Json,\n    routing::{get, post},\n    Router,\n};\nuse serde::{Deserialize, Serialize};\n\n#[derive(Deserialize)]\nstruct ${1:QueryParams} {\n    ${2:page}: Option<u32>,\n    ${3:limit}: Option<u32>,\n}\n\n#[derive(Serialize)]\nstruct ${4:Response} {\n    ${5:data}: Vec<${6:Item}>,\n    ${7:total}: u32,\n}\n\nasync fn ${8:handler}(\n    Path(${9:id}): Path<u32>,\n    Query(params): Query<${1:QueryParams}>,\n    State(${10:state}): State<${11:AppState}>,\n) -> Result<Json<${4:Response}>, StatusCode> {\n    // Handler implementation\n    let response = ${4:Response} {\n        ${5:data}: vec![],\n        ${7:total}: 0,\n    };\n    Ok(Json(response))\n}"),
            
            ("reqwest_client", "use reqwest::Client;\nuse serde::{Deserialize, Serialize};\n\n#[derive(Serialize)]\nstruct ${1:RequestBody} {\n    ${2:field}: String,\n}\n\n#[derive(Deserialize)]\nstruct ${3:ResponseBody} {\n    ${4:result}: String,\n}\n\nasync fn ${5:api_call}() -> Result<${3:ResponseBody}, Box<dyn std::error::Error>> {\n    let client = Client::new();\n    let body = ${1:RequestBody} {\n        ${2:field}: \"${6:value}\".to_string(),\n    };\n    \n    let response = client\n        .post(\"${7:https://api.example.com/endpoint}\")\n        .header(\"Content-Type\", \"application/json\")\n        .json(&body)\n        .send()\n        .await?\n        .json::<${3:ResponseBody}>()\n        .await?;\n    \n    Ok(response)\n}"),
            
            // Database patterns
            ("sqlx_query", "use sqlx::{Row, PgPool};\n\n#[derive(sqlx::FromRow)]\nstruct ${1:User} {\n    id: i32,\n    name: String,\n    email: String,\n}\n\nasync fn ${2:get_user}(pool: &PgPool, user_id: i32) -> Result<Option<${1:User}>, sqlx::Error> {\n    let user = sqlx::query_as::<_, ${1:User}>(\n        \"SELECT id, name, email FROM users WHERE id = $1\"\n    )\n    .bind(user_id)\n    .fetch_optional(pool)\n    .await?;\n    \n    Ok(user)\n}\n\nasync fn ${3:create_user}(pool: &PgPool, name: &str, email: &str) -> Result<${1:User}, sqlx::Error> {\n    let user = sqlx::query_as::<_, ${1:User}>(\n        \"INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email\"\n    )\n    .bind(name)\n    .bind(email)\n    .fetch_one(pool)\n    .await?;\n    \n    Ok(user)\n}"),
            
            ("diesel_model", "use diesel::prelude::*;\nuse serde::{Deserialize, Serialize};\n\n#[derive(Queryable, Selectable, Serialize)]\n#[diesel(table_name = ${1:users})]\n#[diesel(check_for_backend(diesel::pg::Pg))]\npub struct ${2:User} {\n    pub id: i32,\n    pub name: String,\n    pub email: String,\n}\n\n#[derive(Insertable, Deserialize)]\n#[diesel(table_name = ${1:users})]\npub struct ${3:NewUser} {\n    pub name: String,\n    pub email: String,\n}\n\nimpl ${2:User} {\n    pub fn create(conn: &mut PgConnection, new_user: ${3:NewUser}) -> QueryResult<${2:User}> {\n        diesel::insert_into(${1:users}::table)\n            .values(&new_user)\n            .returning(${2:User}::as_returning())\n            .get_result(conn)\n    }\n    \n    pub fn find_by_id(conn: &mut PgConnection, user_id: i32) -> QueryResult<${2:User}> {\n        ${1:users}::table.find(user_id).first(conn)\n    }\n}"),
            
            // Configuration and CLI patterns
            ("clap_cli", "use clap::{Parser, Subcommand};\n\n#[derive(Parser)]\n#[command(name = \"${1:myapp}\")]\n#[command(about = \"${2:A CLI application}\", long_about = None)]\nstruct ${3:Cli} {\n    #[arg(short, long, value_name = \"FILE\")]\n    config: Option<std::path::PathBuf>,\n    \n    #[arg(short, long, action = clap::ArgAction::Count)]\n    verbose: u8,\n    \n    #[command(subcommand)]\n    command: Option<${4:Commands}>,\n}\n\n#[derive(Subcommand)]\nenum ${4:Commands} {\n    ${5:Start} {\n        #[arg(short, long)]\n        port: Option<u16>,\n    },\n    ${6:Stop},\n}\n\nfn main() {\n    let cli = ${3:Cli}::parse();\n    \n    match &cli.command {\n        Some(${4:Commands}::${5:Start} { port }) => {\n            let port = port.unwrap_or(8080);\n            println!(\"Starting server on port {}\", port);\n        }\n        Some(${4:Commands}::${6:Stop}) => {\n            println!(\"Stopping server\");\n        }\n        None => {\n            println!(\"No command specified\");\n        }\n    }\n}"),
            
            ("config_struct", "use serde::{Deserialize, Serialize};\nuse std::fs;\n\n#[derive(Serialize, Deserialize, Debug, Clone)]\npub struct ${1:Config} {\n    pub ${2:server}: ${3:ServerConfig},\n    pub ${4:database}: ${5:DatabaseConfig},\n    pub ${6:logging}: ${7:LoggingConfig},\n}\n\n#[derive(Serialize, Deserialize, Debug, Clone)]\npub struct ${3:ServerConfig} {\n    pub host: String,\n    pub port: u16,\n}\n\n#[derive(Serialize, Deserialize, Debug, Clone)]\npub struct ${5:DatabaseConfig} {\n    pub url: String,\n    pub max_connections: u32,\n}\n\n#[derive(Serialize, Deserialize, Debug, Clone)]\npub struct ${7:LoggingConfig} {\n    pub level: String,\n    pub file: Option<String>,\n}\n\nimpl ${1:Config} {\n    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {\n        let contents = fs::read_to_string(path)?;\n        let config: ${1:Config} = toml::from_str(&contents)?;\n        Ok(config)\n    }\n    \n    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {\n        let config = envy::from_env::<${1:Config}>()?;\n        Ok(config)\n    }\n}"),
            
            // Performance and optimization patterns
            ("benchmarks", "use criterion::{black_box, criterion_group, criterion_main, Criterion};\n\nfn ${1:benchmark_function}(c: &mut Criterion) {\n    c.bench_function(\"${2:test_name}\", |b| {\n        b.iter(|| {\n            // Code to benchmark\n            ${3:expensive_operation}(black_box(${4:input}))\n        })\n    });\n}\n\nfn ${5:comparison_benchmark}(c: &mut Criterion) {\n    let mut group = c.benchmark_group(\"${6:group_name}\");\n    \n    group.bench_function(\"${7:method1}\", |b| {\n        b.iter(|| ${8:method1}(black_box(${4:input})))\n    });\n    \n    group.bench_function(\"${9:method2}\", |b| {\n        b.iter(|| ${10:method2}(black_box(${4:input})))\n    });\n    \n    group.finish();\n}\n\ncriterion_group!(benches, ${1:benchmark_function}, ${5:comparison_benchmark});\ncriterion_main!(benches);"),
            
            ("profiling", "use std::time::Instant;\n\nfn ${1:timed_function}<T, F>(f: F) -> (T, std::time::Duration)\nwhere\n    F: FnOnce() -> T,\n{\n    let start = Instant::now();\n    let result = f();\n    let duration = start.elapsed();\n    (result, duration)\n}\n\n// Usage example\nlet (result, duration) = ${1:timed_function}(|| {\n    ${2:expensive_operation}()\n});\n\nprintln!(\"Operation took: {:?}\", duration);"),
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
            
            // New advanced pattern documentation
            "state_machine" => "State Machine Pattern - Implement finite state machines\n\nUseful for modeling transitions between defined states".to_string(),
            "observer" => "Observer Pattern - Implement event notification system\n\nAllows objects to be notified of changes in other objects".to_string(),
            "command" => "Command Pattern - Encapsulate requests as objects\n\nEnables undo operations and request queuing".to_string(),
            "generic_struct" => "Generic Struct - Create type-parameterized structures\n\nAllows code reuse across different types".to_string(),
            "lifetime_struct" => "Lifetime Struct - Manage borrowed data lifetimes\n\nEnsures references remain valid during struct lifetime".to_string(),
            "where_clause" => "Where Clause - Specify complex trait bounds\n\nMore readable than inline trait bounds for complex generics".to_string(),
            "associated_types" => "Associated Types - Define type relationships in traits\n\nUseful for traits that work with specific associated types".to_string(),
            "channel" => "Async Channel - Communication between async tasks\n\nTokens mpsc channel for message passing".to_string(),
            "actor" => "Actor Pattern - Message-based concurrent computation\n\nEncapsulates state and behavior in independent actors".to_string(),
            "stream" => "Async Stream - Process sequences asynchronously\n\nStream-based data processing with async/await".to_string(),
            "custom_error" => "Custom Error Type - Define application-specific errors\n\nImplements Error trait for better error handling".to_string(),
            "anyhow_error" => "Anyhow Error Handling - Simplified error management\n\nProvides easy error handling with context".to_string(),
            "thiserror" => "ThisError Derive - Automatic Error implementation\n\nDerive macro for Error trait implementation".to_string(),
            "axum_handler" => "Axum Web Handler - HTTP request handler function\n\nAsync web handler with extractors and response".to_string(),
            "reqwest_client" => "HTTP Client - Make HTTP requests with reqwest\n\nAsync HTTP client for API communication".to_string(),
            "sqlx_query" => "SQLx Database Query - Type-safe database operations\n\nAsync database queries with compile-time verification".to_string(),
            "diesel_model" => "Diesel ORM Model - Database model with ORM\n\nType-safe ORM operations for database interactions".to_string(),
            "clap_cli" => "CLI Application - Command-line interface with clap\n\nPowerful CLI parsing with subcommands".to_string(),
            "config_struct" => "Configuration Structure - Application configuration\n\nSerialization-ready config with multiple sources".to_string(),
            "benchmarks" => "Criterion Benchmarks - Performance measurement\n\nMicro-benchmarking with statistical analysis".to_string(),
            "profiling" => "Performance Profiling - Measure execution time\n\nSimple timing utilities for performance analysis".to_string(),
            
            // Numeric types
            "i8" => "i8 - 8-bit signed integer\n\nRange: -128 to 127".to_string(),
            "i16" => "i16 - 16-bit signed integer\n\nRange: -32,768 to 32,767".to_string(),
            "i32" => "i32 - 32-bit signed integer\n\nRange: -2,147,483,648 to 2,147,483,647".to_string(),
            "i64" => "i64 - 64-bit signed integer\n\nLarge integer type".to_string(),
            "u8" => "u8 - 8-bit unsigned integer\n\nRange: 0 to 255, commonly used for bytes".to_string(),
            "u16" => "u16 - 16-bit unsigned integer\n\nRange: 0 to 65,535".to_string(),
            "u32" => "u32 - 32-bit unsigned integer\n\nRange: 0 to 4,294,967,295".to_string(),
            "u64" => "u64 - 64-bit unsigned integer\n\nLarge unsigned integer type".to_string(),
            "usize" => "usize - Pointer-sized unsigned integer\n\nUsed for array indices and memory sizes".to_string(),
            "isize" => "isize - Pointer-sized signed integer\n\nSize depends on target architecture".to_string(),
            "f32" => "f32 - 32-bit floating point\n\nSingle-precision floating point".to_string(),
            "f64" => "f64 - 64-bit floating point\n\nDouble-precision floating point (default)".to_string(),
            "bool" => "bool - Boolean type\n\nValues: true or false".to_string(),
            "char" => "char - Unicode scalar value\n\n4-byte Unicode character".to_string(),
            "str" => "str - String slice\n\nBorrowed string content, usually seen as &str".to_string(),
            
            // Collections
            "HashMap" => "HashMap<K, V> - Hash table implementation\n\nFast key-value lookups".to_string(),
            "HashSet" => "HashSet<T> - Set based on HashMap\n\nUnique elements with fast lookups".to_string(),
            "BTreeMap" => "BTreeMap<K, V> - Sorted map\n\nMaintains keys in sorted order".to_string(),
            "BTreeSet" => "BTreeSet<T> - Sorted set\n\nMaintains elements in sorted order".to_string(),
            "VecDeque" => "VecDeque<T> - Double-ended queue\n\nEfficient push/pop from both ends".to_string(),
            
            // Important traits
            "Iterator" => "Iterator trait - Provides iteration capabilities\n\nDefines next() method for sequential access".to_string(),
            "Clone" => "Clone trait - Explicit duplication\n\nDefines clone() method for deep copying".to_string(),
            "Copy" => "Copy trait - Implicit duplication\n\nFor types that can be copied with memcpy".to_string(),
            "Send" => "Send trait - Safe to transfer across threads\n\nMarker trait for thread-safe ownership transfer".to_string(),
            "Sync" => "Sync trait - Safe to share between threads\n\nMarker trait for thread-safe reference sharing".to_string(),
            "Drop" => "Drop trait - Custom destructor\n\nDefines drop() method called when value goes out of scope".to_string(),
            
            // Async types
            "Future" => "Future trait - Asynchronous computation\n\nDefines poll() method for async execution".to_string(),
            "Pin" => "Pin<P> - Prevents moving of pinned data\n\nRequired for self-referential futures".to_string(),
            
            // Error handling
            "Error" => "Error trait - Standard error interface\n\nProvides source() and description() methods".to_string(),
            "Some" => "Option::Some - Contains a value\n\nWraps a value in Option enum".to_string(),
            "None" => "Option::None - No value present\n\nRepresents absence of value".to_string(),
            "Ok" => "Result::Ok - Success variant\n\nContains the success value".to_string(),
            "Err" => "Result::Err - Error variant\n\nContains the error value".to_string(),
            
            // Memory management
            "Cell" => "Cell<T> - Interior mutability for Copy types\n\nAllows mutation through shared references".to_string(),
            "Weak" => "Weak<T> - Non-owning reference to Arc/Rc\n\nBreaks reference cycles".to_string(),
            
            // Time types
            "Duration" => "Duration - Time span\n\nRepresents a length of time".to_string(),
            "Instant" => "Instant - Monotonic timestamp\n\nFor measuring elapsed time".to_string(),
            
            // File and path types
            "Path" => "Path - Borrowed file system path\n\nImmutable reference to a filesystem path".to_string(),
            "PathBuf" => "PathBuf - Owned file system path\n\nOwned, mutable filesystem path".to_string(),
            "File" => "File - Handle to an open file\n\nFor reading and writing files".to_string(),
            
            // Common macros
            "assert!" => "assert! macro - Runtime assertion\n\nPanics if condition is false".to_string(),
            "assert_eq!" => "assert_eq! macro - Equality assertion\n\nPanics if values are not equal".to_string(),
            "assert_ne!" => "assert_ne! macro - Inequality assertion\n\nPanics if values are equal".to_string(),
            "panic!" => "panic! macro - Deliberate program termination\n\nCauses the program to abort".to_string(),
            "todo!" => "todo! macro - Placeholder for unfinished code\n\nPanics with 'not yet implemented' message".to_string(),
            "unimplemented!" => "unimplemented! macro - Placeholder for missing implementation\n\nPanics with 'not implemented' message".to_string(),
            "dbg!" => "dbg! macro - Debug print and return value\n\nPrints value and returns it".to_string(),
            
            // Attributes
            "derive" => "#[derive] attribute - Auto-generate trait implementations\n\nExample: #[derive(Debug, Clone)]".to_string(),
            "test" => "#[test] attribute - Mark function as test\n\nUsed with cargo test".to_string(),
            "inline" => "#[inline] attribute - Inline function hint\n\nSuggests compiler to inline function".to_string(),
            
            // Common methods
            "new" => "new() - Constructor function\n\nConvention for creating new instances".to_string(),
            "len" => "len() - Get length/count\n\nReturns number of elements".to_string(),
            "is_empty" => "is_empty() - Check if empty\n\nReturns true if length is zero".to_string(),
            "push" => "push() - Add element to end\n\nFor Vec, String, etc.".to_string(),
            "pop" => "pop() - Remove element from end\n\nReturns Option<T>".to_string(),
            "collect" => "collect() - Consume iterator into collection\n\nBuilds collection from iterator".to_string(),
            "map" => "map() - Transform elements\n\nApplies function to each element".to_string(),
            "filter" => "filter() - Select elements\n\nKeeps elements matching predicate".to_string(),
            "find" => "find() - Find first matching element\n\nReturns Option<T>".to_string(),
            "any" => "any() - Check if any element matches\n\nReturns boolean".to_string(),
            "all" => "all() - Check if all elements match\n\nReturns boolean".to_string(),
            "unwrap" => "unwrap() - Extract value or panic\n\nFor Option and Result".to_string(),
            "expect" => "expect() - Extract value or panic with message\n\nFor Option and Result".to_string(),
            "unwrap_or" => "unwrap_or() - Extract value or use default\n\nFor Option and Result".to_string(),
            "is_some" => "is_some() - Check if Option has value\n\nReturns boolean".to_string(),
            "is_none" => "is_none() - Check if Option is empty\n\nReturns boolean".to_string(),
            "is_ok" => "is_ok() - Check if Result is success\n\nReturns boolean".to_string(),
            "is_err" => "is_err() - Check if Result is error\n\nReturns boolean".to_string(),
            
            _ => format!("{} - Rust keyword/identifier", keyword),
        }
    }
}
