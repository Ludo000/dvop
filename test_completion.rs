// Enhanced test file for wide completion with documentation
// Use Ctrl+Space to trigger completion and see improved documentation display

fn main() {
    // Try typing these and press Ctrl+Space to see wider documentation:
    // f  <- should show "fn" with detailed function documentation
    // let  <- should show let keyword with variable binding explanation
    // Vec  <- should show Vec type with growable array description
    // matc  <- should show match with pattern matching explanation
    // async  <- should show async with asynchronous function details
    // Result  <- should show Result<T,E> with error handling info
    // Option  <- should show Option<T> with optional value details
    
    let mut vector = Vec::new();
    vector.push(42);
    
    let option_value = Some(vector.len());
    match option_value {
        Some(length) => println!("Vector length: {}", length),
        None => println!("No value"),
    }
    
    // Test more completion features for wide documentation:
    // HashMap  <- should show hash table implementation details
    // Arc  <- should show atomic reference counter explanation
    // Mutex  <- should show mutual exclusion primitive details
    // async_fn  <- should show async function snippet with explanation
    // custom_error  <- should show custom error pattern with full details
    
    let result: Result<i32, &str> = Ok(42);
    if let Ok(value) = result {
        println!("Success: {}", value);
    }
}

async fn async_example() {
    // Test async-related completions with detailed docs:
    // await  <- should show await keyword with async operation details
    // Future  <- should show Future trait with asynchronous computation info
    // tokio  <- should show tokio async runtime details
    // Stream  <- should show async stream processing details
    // channel  <- should show async channel communication details
    
    println!("Async function example");
}

// Test struct and implementation completions:
struct ExampleStruct {
    field: String,
}

impl ExampleStruct {
    // Try completion here for:
    // new  <- constructor function convention
    // clone  <- Clone trait method
    // default  <- Default trait implementation
    
    fn new(value: String) -> Self {
        Self { field: value }
    }
}

// Test trait completion:
trait ExampleTrait {
    // Try completion for:
    // Associated  <- associated types explanation
    // where  <- where clause for complex trait bounds
    
    fn example_method(&self);
}

#[derive(Debug, Clone)]  // Try completion after #[derive(
struct DerivedStruct {
    data: Vec<i32>,
}

// Test error handling patterns:
fn error_handling_example() -> Result<(), Box<dyn std::error::Error>> {
    // Try completion for:
    // unwrap  <- extract value or panic explanation
    // expect  <- extract value or panic with message
    // map_err  <- transform error type explanation
    // and_then  <- chain result operations explanation
    
    Ok(())
}
