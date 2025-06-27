# Rust Code Snippets and Keywords for Text Editor

This document lists all the available Rust code snippets and keywords in the text editor completion system.

## Core Language Keywords
- `as` - Type casting and trait disambiguation
- `async` - Asynchronous function definition
- `await` - Wait for async operation
- `break` - Exit from loop
- `const` - Compile-time constant
- `continue` - Skip to next loop iteration
- `crate` - Reference to current crate
- `dyn` - Dynamic trait object
- `else` - Alternative branch
- `enum` - Enumeration definition
- `extern` - External block or function
- `false` - Boolean false value
- `fn` - Function definition
- `for` - For loop
- `if` - Conditional statement
- `impl` - Implementation block
- `in` - Part of for loop syntax
- `let` - Variable binding
- `loop` - Infinite loop
- `match` - Pattern matching
- `mod` - Module definition
- `move` - Closure ownership
- `mut` - Mutable modifier
- `pub` - Public visibility
- `ref` - Reference binding
- `return` - Early return
- `self` - Method receiver
- `Self` - Type alias for current type
- `static` - Static variable
- `struct` - Structure definition
- `super` - Parent module
- `trait` - Trait definition
- `true` - Boolean true value
- `type` - Type alias
- `unsafe` - Unsafe code block
- `use` - Import declaration
- `where` - Where clause for generics
- `while` - While loop

## Reserved Keywords (Future Use)
- `abstract`, `become`, `box`, `do`, `final`, `macro`, `override`, `priv`, `typeof`, `unsized`, `virtual`, `yield`, `try`, `union`

## Primitive Types
- `i8`, `i16`, `i32`, `i64`, `i128`, `isize` - Signed integers
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize` - Unsigned integers
- `f32`, `f64` - Floating point numbers
- `bool` - Boolean type
- `char` - Unicode character
- `str` - String slice

## Common Collection Types
- `String` - Owned string
- `Vec` - Growable array
- `HashMap` - Hash table
- `HashSet` - Set based on HashMap
- `BTreeMap` - Sorted map
- `BTreeSet` - Sorted set
- `LinkedList` - Doubly-linked list
- `VecDeque` - Double-ended queue
- `BinaryHeap` - Priority queue

## Smart Pointers and Memory Management
- `Box` - Heap allocation
- `Arc` - Atomic reference counter
- `Rc` - Reference counter (single-threaded)
- `RefCell` - Interior mutability with runtime checks
- `Cell` - Interior mutability for Copy types
- `Mutex` - Mutual exclusion
- `RwLock` - Reader-writer lock
- `Weak` - Non-owning reference
- `Pin` - Prevents moving
- `ManuallyDrop` - Manual drop control
- `MaybeUninit` - Potentially uninitialized data

## Option and Result Types
- `Option` - Optional value type
- `Some` - Contains a value
- `None` - No value present
- `Result` - Error handling type
- `Ok` - Success variant
- `Err` - Error variant

## Essential Traits
- `Clone` - Explicit duplication
- `Copy` - Implicit duplication
- `Debug` - Debug formatting
- `Default` - Default value constructor
- `PartialEq`, `Eq` - Equality comparison
- `PartialOrd`, `Ord` - Ordering comparison
- `Hash` - Hash value computation
- `Display` - User-facing formatting
- `From`, `Into` - Value conversion
- `TryFrom`, `TryInto` - Fallible conversion
- `AsRef`, `AsMut` - Reference conversion
- `Borrow`, `BorrowMut` - Borrowing
- `Deref`, `DerefMut` - Dereference operations
- `Drop` - Custom destructor
- `Send` - Thread-safe ownership transfer
- `Sync` - Thread-safe reference sharing
- `Sized` - Known size at compile time
- `Unpin` - Safe to move when pinned

## Iterator Traits and Types
- `Iterator` - Core iteration trait
- `IntoIterator` - Convert to iterator
- `DoubleEndedIterator` - Bidirectional iteration
- `ExactSizeIterator` - Known length iteration
- `Extend` - Extend with iterator
- `FromIterator` - Construct from iterator

## Function Traits
- `Fn` - Immutable closure
- `FnMut` - Mutable closure
- `FnOnce` - Consuming closure

## Async/Future Types
- `Future` - Asynchronous computation
- `Poll` - Future execution state
- `Ready` - Future is complete
- `Pending` - Future needs more time
- `Context` - Async execution context
- `Waker` - Wake async task

## File and Path Types
- `Path` - Borrowed filesystem path
- `PathBuf` - Owned filesystem path
- `File` - File handle
- `OpenOptions` - File opening options
- `DirEntry` - Directory entry
- `ReadDir` - Directory iterator
- `Metadata` - File metadata
- `Permissions` - File permissions

## IO Traits and Types
- `Read` - Reading trait
- `Write` - Writing trait
- `BufRead` - Buffered reading
- `BufReader` - Buffered reader
- `BufWriter` - Buffered writer
- `Cursor` - In-memory IO
- `Stdin`, `Stdout`, `Stderr` - Standard streams

## Threading and Concurrency
- `Thread` - Thread handle
- `ThreadId` - Thread identifier
- `JoinHandle` - Thread join handle
- `Builder` - Thread builder
- `LocalKey` - Thread-local storage
- `Barrier` - Thread synchronization barrier
- `Condvar` - Condition variable
- `Once` - One-time initialization
- `OnceWith` - One-time initialization with value

## Atomic Types
- `AtomicBool`, `AtomicI8`, `AtomicI16`, `AtomicI32`, `AtomicI64`, `AtomicIsize`
- `AtomicU8`, `AtomicU16`, `AtomicU32`, `AtomicU64`, `AtomicUsize`
- `AtomicPtr` - Atomic pointer
- `Ordering` - Memory ordering
- `Relaxed`, `Acquire`, `Release`, `AcqRel`, `SeqCst` - Ordering variants

## Time Types
- `Duration` - Time span
- `Instant` - Monotonic timestamp
- `SystemTime` - System clock timestamp
- `UNIX_EPOCH` - Unix timestamp epoch

## Network Types
- `TcpListener`, `TcpStream` - TCP networking
- `UdpSocket` - UDP networking
- `IpAddr`, `Ipv4Addr`, `Ipv6Addr` - IP addresses
- `SocketAddr`, `SocketAddrV4`, `SocketAddrV6` - Socket addresses
- `ToSocketAddrs` - Address resolution

## Process Types
- `Command` - Process builder
- `Child` - Running process
- `ChildStdin`, `ChildStdout`, `ChildStderr` - Child process streams
- `ExitStatus` - Process exit status
- `Output` - Process output
- `Stdio` - Standard IO configuration

## Essential Macros
- `println!`, `eprintln!` - Print to stdout/stderr
- `print!`, `eprint!` - Print without newline
- `format!`, `format_args!` - String formatting
- `vec!` - Vector creation
- `panic!` - Program termination
- `assert!`, `assert_eq!`, `assert_ne!` - Assertions
- `debug_assert!`, `debug_assert_eq!`, `debug_assert_ne!` - Debug assertions
- `unreachable!` - Mark unreachable code
- `unimplemented!` - Placeholder for missing code
- `todo!` - Placeholder for future code
- `compile_error!` - Compile-time error
- `concat!`, `stringify!` - String operations
- `include!`, `include_str!`, `include_bytes!` - File inclusion
- `env!`, `option_env!` - Environment variables
- `cfg!` - Configuration checking
- `line!`, `column!`, `file!`, `module_path!` - Source location
- `matches!` - Pattern matching test
- `dbg!` - Debug print and return
- `write!`, `writeln!` - Formatted writing
- `try!` - Error propagation (deprecated)
- `macro_rules!` - Macro definition

## Attributes
- `#[allow]`, `#[warn]`, `#[deny]`, `#[forbid]` - Lint control
- `#[deprecated]` - Deprecation warning
- `#[must_use]` - Warning if unused
- `#[repr]` - Data representation
- `#[derive]` - Trait derivation
- `#[cfg]`, `#[cfg_attr]` - Conditional compilation
- `#[test]` - Test function
- `#[bench]` - Benchmark function
- `#[should_panic]` - Expected panic test
- `#[ignore]` - Ignored test
- `#[inline]` - Inlining hint
- `#[cold]` - Cold path hint
- `#[target_feature]` - Target-specific features
- `#[no_mangle]` - Prevent name mangling
- `#[export_name]`, `#[link_name]` - Symbol names
- `#[link]` - Link to external library
- `#[used]` - Prevent removal
- `#[crate_type]` - Crate type
- `#[no_main]`, `#[no_std]` - Special crate configuration
- `#[macro_use]`, `#[macro_export]` - Macro control
- `#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]` - Procedural macros

## Common Method Names
- `new` - Constructor
- `len`, `is_empty` - Size operations
- `push`, `pop` - Stack operations
- `get`, `get_mut` - Safe access
- `insert`, `remove` - Collection modification
- `clear` - Remove all elements
- `contains` - Membership test
- `iter`, `into_iter` - Iteration
- `collect` - Build from iterator
- `map`, `filter`, `fold` - Iterator adaptors
- `find`, `any`, `all` - Search operations
- `count`, `enumerate`, `zip` - Iterator utilities
- `take`, `skip`, `rev` - Iterator control
- `sort`, `reverse` - Ordering operations
- `clone` - Duplication
- `unwrap`, `expect` - Extract values
- `unwrap_or`, `unwrap_or_else` - Safe extraction
- `is_some`, `is_none` - Option checks
- `is_ok`, `is_err` - Result checks

## Popular Third-Party Crates
- `serde` - Serialization framework
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `clap` - Command-line parsing
- `anyhow`, `thiserror` - Error handling
- `log`, `env_logger`, `tracing` - Logging
- `chrono` - Date and time
- `uuid` - UUID generation
- `regex` - Regular expressions
- `rayon` - Data parallelism
- `crossbeam` - Concurrency utilities
- `dashmap` - Concurrent HashMap
- `once_cell`, `lazy_static` - Lazy initialization
- `parking_lot` - Efficient synchronization
- `flume` - Multi-producer, multi-consumer channels
- `criterion` - Benchmarking
- `proptest`, `quickcheck` - Property testing
- `mockall`, `wiremock` - Testing utilities

## Basic Language Constructs
- `fn` - Function definition
- `struct` - Structure definition
- `impl` - Implementation block
- `match` - Pattern matching
- `if` - If statement
- `iflet` - If-let pattern
- `for` - For loop
- `while` - While loop
- `loop` - Infinite loop

## Functions and Main
- `main` - Basic main function
- `mainargs` - Main with error handling
- `async_main` - Async main with Tokio
- `pub_fn` - Public function
- `async_fn` - Async function

## Error Handling
- `result` - Result type
- `option` - Option type
- `unwrap_or` - Unwrap with default
- `unwrap_or_else` - Unwrap with closure
- `map_err` - Map error transformation
- `try_block` - Try block pattern
- `error_match` - Match on Result
- `custom_error` - Custom error type definition
- `anyhow_error` - Anyhow error handling
- `thiserror` - ThisError derive macro

## Collections and Iterators
- `vec_new` - New vector
- `vec_with` - Vector with values
- `hashmap` - HashMap creation
- `hashset` - HashSet creation
- `collect` - Collect iterator
- `filter_map` - Filter and map
- `fold` - Fold operation
- `enumerate` - Enumerate iterator
- `map` - Map transformation
- `filter` - Filter operation
- `find` - Find element
- `any` - Any condition
- `all` - All condition

## Async/Await Patterns
- `async_block` - Async block
- `spawn` - Tokio spawn
- `join` - Tokio join
- `select` - Tokio select
- `timeout` - Async timeout
- `channel` - Async channel
- `actor` - Actor pattern
- `stream` - Async stream processing

## Testing
- `test` - Test function
- `test_async` - Async test
- `assert_eq` - Assert equal
- `assert_ne` - Assert not equal
- `assert` - Assert condition
- `panic` - Panic macro
- `should_panic` - Should panic test

## Structs and Traits
- `derive` - Derive attributes
- `enum` - Enum definition
- `trait` - Trait definition
- `impl_trait` - Trait implementation
- `impl_default` - Default implementation
- `impl_display` - Display implementation

## Memory Management
- `box` - Box allocation
- `rc` - Reference counted
- `arc` - Atomic reference counted
- `refcell` - RefCell wrapper
- `mutex` - Mutex wrapper
- `rwlock` - Read-write lock
- `weak` - Weak reference

## Closures and Functional Programming
- `closure` - Basic closure
- `move_closure` - Move closure

## Modules and Imports
- `mod` - Module definition
- `use` - Use statement
- `use_std` - Use std library
- `extern_crate` - External crate
- `pub_mod` - Public module
- `pub_use` - Public use

## Macros
- `macro_rules` - Macro definition
- `println` - Print line
- `eprintln` - Error print line
- `format` - Format string
- `dbg` - Debug print
- `todo` - Todo placeholder
- `unimplemented` - Unimplemented placeholder

## File I/O and Serialization
- `read_file` - Read file to string
- `write_file` - Write string to file
- `open_file` - Open file handle
- `create_file` - Create file handle
- `serde_derive` - Serde serialization

## Network and HTTP
- `http_get` - HTTP GET request
- `http_post` - HTTP POST request
- `reqwest_client` - HTTP client with reqwest

## Web Framework (Axum)
- `axum_handler` - Axum HTTP handler

## Database
- `sqlx_query` - SQLx database query
- `diesel_model` - Diesel ORM model

## Configuration and CLI
- `clap_cli` - CLI application with clap
- `config_struct` - Configuration structure

## Performance and Optimization
- `benchmarks` - Criterion benchmarks
- `profiling` - Performance profiling

## Advanced Patterns
- `state_machine` - Finite state machine
- `observer` - Observer pattern
- `command` - Command pattern
- `builder` - Builder pattern
- `singleton` - Singleton pattern
- `new_type` - New type pattern

## Generic and Lifetime Patterns
- `generic_struct` - Generic structure
- `lifetime_struct` - Lifetime structure
- `where_clause` - Where clause
- `associated_types` - Associated types

## Usage

To use these snippets in the text editor:

1. **Manual trigger**: Press `Ctrl+Space` or `F1` to show completion suggestions
2. **Auto-trigger**: Type `.`, `::`, or `->` to automatically show relevant completions
3. **Type-based**: Start typing a snippet name (e.g., "fn", "struct") to filter suggestions

The completion system will show:
- **Keywords** with a text icon
- **Snippets** with a script icon  
- **Buffer words** with a file icon

All snippets include placeholder variables (${1:name}) that can be filled in when inserted.
