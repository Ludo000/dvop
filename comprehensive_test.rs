// Comprehensive test for wide documentation display in completion popup
// Use Ctrl+Space to trigger completion and see extensive documentation

fn main() {
    // Test keywords with detailed documentation - try Ctrl+Space after typing:
    // Vec   <- Vec<T> - A growable array type with detailed explanation
    // Hash  <- HashMap, HashSet with comprehensive hash table descriptions  
    // Arc   <- Arc<T> - Atomic reference counter for shared ownership with thread safety details
    // Mutex <- Mutex<T> - Mutual exclusion primitive for thread safety with comprehensive locking explanation
    // async <- async keyword - Define asynchronous function with detailed async/await explanation
    // await <- await keyword - Wait for async operation with comprehensive future handling details
    
    let vector = Vec::new();
    let hashmap = std::collections::HashMap::new();
    
    // Test Result and Option with comprehensive error handling documentation:
    let result: Result<i32, &str> = Ok(42);
    let option: Option<String> = Some("test".to_string());
    
    match result {
        Ok(value) => println!("Success with comprehensive match pattern explanation: {}", value),
        Err(error) => println!("Error with detailed error handling: {}", error),
    }
    
    // Test snippets with extensive template documentation:
    // custom_error  <- Complete error type definition with Display, Error trait implementations
    // channel       <- Tokio async channel setup with comprehensive message passing explanation
    // actor         <- Actor pattern implementation with detailed message handling and state management
    // builder       <- Builder pattern with comprehensive fluent API and validation explanations
    // benchmarks    <- Criterion benchmark setup with detailed performance measurement configuration
}

async fn async_comprehensive_test() {
    // Test async-related completions with extensive documentation:
    // Future  <- Future trait with comprehensive asynchronous computation explanation and poll mechanism
    // Stream  <- Async stream processing with detailed iterator-like interface for async sequences
    // tokio   <- Tokio async runtime with comprehensive task scheduling and I/O explanation
    // select  <- Tokio select macro with detailed concurrent future execution explanation
    // join    <- Tokio join operations with comprehensive parallel execution explanation
    
    println!("Async function with extensive async/await documentation examples");
}

struct ComprehensiveStruct {
    field_with_long_name: String,
    another_detailed_field: Vec<i32>,
}

impl ComprehensiveStruct {
    // Test method completions with detailed documentation:
    // new    <- Constructor function convention with comprehensive instance creation explanation
    // clone  <- Clone trait method with detailed deep copying mechanism explanation  
    // len    <- Length/count method with comprehensive collection size explanation
    // push   <- Add element method with detailed vector growth and capacity explanation
    // collect <- Iterator collection with comprehensive transformation and allocation explanation
    
    fn new() -> Self {
        Self {
            field_with_long_name: String::new(),
            another_detailed_field: Vec::new(),
        }
    }
}

// Test trait definitions with comprehensive documentation:
trait ComprehensiveTrait {
    // Test associated types and where clauses with detailed explanations:
    // where      <- Where clause for complex trait bounds with comprehensive generic constraints
    // associated <- Associated types with detailed type relationships in traits explanation
    // Send       <- Send trait with comprehensive thread safety and ownership transfer explanation
    // Sync       <- Sync trait with detailed shared reference thread safety explanation
    
    type AssociatedType;
    fn comprehensive_method(&self) -> Self::AssociatedType;
}

// Test derive macros with extensive trait implementation documentation:
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DerivedStruct {
    comprehensive_data: Vec<String>,
    detailed_metadata: std::collections::HashMap<String, i32>,
}

// Test error handling patterns with comprehensive documentation:
fn comprehensive_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test error methods with detailed documentation:
    // unwrap_or      <- Extract value or use default with comprehensive Option/Result handling
    // map_err        <- Transform error type with detailed error conversion explanation
    // and_then       <- Chain Result operations with comprehensive monadic operation explanation
    // unwrap_or_else <- Extract value or compute default with detailed lazy evaluation explanation
    
    let result = "42".parse::<i32>()?;
    println!("Parsed with comprehensive error propagation: {}", result);
    Ok(())
}

mod comprehensive_module {
    // Test module-level completions with detailed documentation:
    // pub use <- Public re-export with comprehensive module interface explanation
    // super   <- Parent module reference with detailed module hierarchy explanation
    // crate   <- Crate root reference with comprehensive absolute path explanation
    
    pub use std::collections::HashMap as ComprehensiveHashMap;
    
    pub fn comprehensive_function() {
        println!("Function with extensive module documentation");
    }
}
