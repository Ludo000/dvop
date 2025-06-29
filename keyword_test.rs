// Test specific keywords to see what documentation they show
fn main() {
    // Test some basic keywords that should have enhanced documentation
    let x = 5; // "let" keyword
    let mut y = 10; // "mut" keyword
    
    // Test some other keywords
    pub fn test_function() { // "pub" and "fn" keywords
        if true { // "if" keyword
            for i in 0..10 { // "for" keyword
                println!("{}", i);
            }
        }
    }
    
    // Test more complex keywords
    struct TestStruct { // "struct" keyword
        field: String,
    }
    
    impl TestStruct { // "impl" keyword
        fn new() -> Self { // "Self" keyword
            Self {
                field: String::new(),
            }
        }
    }
    
    // Test types that might be showing as keywords
    let s: String = String::new(); // "String" type
    let v: Vec<i32> = Vec::new(); // "Vec" and "i32" types
    let opt: Option<i32> = Some(42); // "Option" and "Some"
}
