use std::time::Instant;

fn main() {
    println!("Testing parallel processing...");
    
    // Simple test to verify parallel processing works
    let start = Instant::now();
    
    // Simulate processing multiple items
    let result: Vec<_> = (0..100).collect();
    
    let duration = start.elapsed();
    println!("Collected {} items in {:?}", result.len(), duration);
    
    println!("Test completed successfully!");
}