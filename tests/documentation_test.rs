// Test to verify documentation and benchmark functionality
#[test]
fn test_documentation_exists() {
    // Verify that documentation files exist
    assert!(std::path::Path::new("docs/PERFORMANCE_OPTIMIZATION.md").exists(), 
           "Performance optimization documentation should exist");
    
    assert!(std::path::Path::new("performance_analysis.md").exists(), 
           "Performance analysis report should exist");
}

#[test]
fn test_benchmark_structure() {
    // Verify that benchmark files are properly structured
    assert!(std::path::Path::new("benches/performance_bench.rs").exists(), 
           "Performance benchmark should exist");
    
    // Verify benchmark content
    let benchmark_content = std::fs::read_to_string("benches/performance_bench.rs")
        .expect("Should be able to read benchmark file");
    
    assert!(benchmark_content.contains("bench_metrics_collection"), 
           "Benchmark should contain metrics collection test");
    assert!(benchmark_content.contains("bench_security_threat_detection"), 
           "Benchmark should contain security threat detection test");
    assert!(benchmark_content.contains("bench_network_monitoring"), 
           "Benchmark should contain network monitoring test");
}

#[test]
fn test_performance_analysis_content() {
    // Verify performance analysis content
    let analysis_content = std::fs::read_to_string("performance_analysis.md")
        .expect("Should be able to read performance analysis file");
    
    assert!(analysis_content.contains("Performance Analysis Report"), 
           "Performance analysis should have proper title");
    assert!(analysis_content.contains("Optimization Opportunities"), 
           "Performance analysis should contain optimization opportunities");
    assert!(analysis_content.contains("Benchmarking"), 
           "Performance analysis should mention benchmarking");
}

#[test]
fn test_documentation_content() {
    // Verify documentation content
    let doc_content = std::fs::read_to_string("docs/PERFORMANCE_OPTIMIZATION.md")
        .expect("Should be able to read performance documentation file");
    
    assert!(doc_content.contains("Performance Optimization Guide"), 
           "Documentation should have proper title");
    assert!(doc_content.contains("Benchmarking"), 
           "Documentation should contain benchmarking section");
    assert!(doc_content.contains("Optimization Techniques"), 
           "Documentation should contain optimization techniques");
    assert!(doc_content.contains("Best Practices"), 
           "Documentation should contain best practices");
}