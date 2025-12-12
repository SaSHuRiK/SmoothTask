use rayon::prelude::*;
use std::time::Instant;

fn main() {
    println!("Testing Rayon parallel processing...");
    
    let start = Instant::now();
    
    // Test parallel processing with Rayon
    let result: Vec<_> = (0..1000)
        .into_par_iter()  // Convert to parallel iterator
        .map(|i| i * 2)   // Simple transformation
        .collect();       // Collect results
    
    let duration = start.elapsed();
    println!("Processed {} items in {:?}", result.len(), duration);
    println!("First few results: {:?}", &result[..5]);
    
    println!("Rayon test completed successfully!");
}