// Benchmark for git-merge-ai-resolver performance

use git_merge_ai_resolver::{
    conflict_parser::{ConflictFile, ConflictRegion},
    resolution_engine::{ResolutionEngine, ResolutionStrategy},
};
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::{Duration, Instant};

// Sample code snippets to use in conflict generation
const CODE_SNIPPETS: [&str; 10] = [
    "function add(a, b) {\n  return a + b;\n}",
    "def process_data(data):\n  return data.filter()",
    "public static void main(String[] args) {\n  System.out.println(\"Hello world\");\n}",
    "import { useState } from 'react';\n\nfunction Counter() {\n  const [count, setCount] = useState(0);\n  return <button onClick={() => setCount(count + 1)}>{count}</button>;\n}",
    "fn calculate_fibonacci(n: u32) -> u32 {\n  match n {\n    0 => 0,\n    1 => 1,\n    _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),\n  }\n}",
    "class User {\n  constructor(name, age) {\n    this.name = name;\n    this.age = age;\n  }\n\n  greet() {\n    return `Hello, ${this.name}`;\n  }\n}",
    "async function fetchData() {\n  const response = await fetch('https://api.example.com/data');\n  return response.json();\n}",
    "const CONFIG = {\n  apiKey: 'your-api-key',\n  endpoint: 'https://api.example.com',\n  timeout: 5000,\n  retries: 3\n};",
    "import React, { useEffect } from 'react';\nimport axios from 'axios';\n\nfunction App() {\n  useEffect(() => {\n    axios.get('/api/data').then(response => {\n      console.log(response.data);\n    });\n  }, []);\n\n  return <div>Loading...</div>;\n}",
    "struct Point {\n  x: f64,\n  y: f64,\n}\n\nimpl Point {\n  fn new(x: f64, y: f64) -> Self {\n    Point { x, y }\n  }\n\n  fn distance(&self, other: &Point) -> f64 {\n    ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()\n  }\n}",
];

// Helper function to generate a random modified version of a code snippet
fn generate_modified_version(original: &str, mut rng: &mut impl Rng) -> String {
    // Randomly choose a modification type
    let modification_type = rng.gen_range(0..4);
    
    match modification_type {
        0 => {
            // Add a comment
            let lines: Vec<&str> = original.lines().collect();
            let position = rng.gen_range(0..lines.len());
            let mut modified = lines.clone();
            modified.insert(position, &format!("// TODO: Add more functionality - {}", rng.gen_range(0..1000)));
            modified.join("\n")
        }
        1 => {
            // Add whitespace changes
            original
                .replace(" {", "\n{")
                .replace("\n", "\n  ")
                .replace("  \n", "\n")
        }
        2 => {
            // Modify existing code slightly
            original
                .replace("return", "const result =")
                .replace(";", ";\n  return result;")
        }
        3 => {
            // Add new functionality
            let additional_function = CODE_SNIPPETS.choose(&mut rng).unwrap();
            format!("{original}\n\n// Added new functionality\n{additional_function}")
        }
        _ => original.to_string(),
    }
}

// Helper function to generate a random conflict file with varying complexity
fn generate_conflict_file(conflict_count: usize, mut rng: &mut impl Rng) -> ConflictFile {
    let mut conflicts = Vec::new();
    let mut content = String::from("// Beginning of file\n\n");
    
    for i in 0..conflict_count {
        // Pick a random code snippet as base
        let base_snippet = CODE_SNIPPETS.choose(&mut rng).unwrap();
        
        // Create modified versions for "ours" and "theirs"
        let our_version = generate_modified_version(base_snippet, &mut rng);
        let their_version = generate_modified_version(base_snippet, &mut rng);
        
        // Create conflict region
        let conflict = ConflictRegion {
            base_content: base_snippet.to_string(),
            our_content: our_version.clone(),
            their_content: their_version.clone(),
            start_line: i * 20 + 3,  // Arbitrary line numbers that increase with each conflict
            end_line: i * 20 + 15,
        };
        
        conflicts.push(conflict);
        
        // Add conflict markers to content
        content.push_str(&format!(
            "// Section {}\n<<<<<<< HEAD\n{}\n=======\n{}\n>>>>>>> feature-branch\n\n",
            i + 1, our_version, their_version
        ));
        
        // Add some non-conflicting content between conflicts
        if i < conflict_count - 1 {
            content.push_str("// Non-conflicting section\nconst VERSION = '1.0.0';\n\n");
        }
    }
    
    content.push_str("// End of file\n");
    
    ConflictFile {
        path: format!("test-file-{}.txt", rng.gen_range(0..1000)),
        conflicts,
        content,
    }
}

// Helper struct to collect benchmark results
#[derive(Debug)]
struct BenchmarkResult {
    conflict_count: usize,
    resolution_time: Duration,
    resolved_conflicts: usize,
    strategy_name: String,
}

// Simple whitespace-only strategy for benchmarking
struct WhitespaceOnlyStrategy;

impl WhitespaceOnlyStrategy {
    fn new() -> Self {
        WhitespaceOnlyStrategy {}
    }
    
    fn normalize_whitespace(&self, s: &str) -> String {
        s.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

impl ResolutionStrategy for WhitespaceOnlyStrategy {
    fn name(&self) -> &str {
        "whitespace-only"
    }
    
    fn can_handle(&self, conflict: &ConflictRegion) -> bool {
        let our_normalized = self.normalize_whitespace(&conflict.our_content);
        let their_normalized = self.normalize_whitespace(&conflict.their_content);
        
        our_normalized == their_normalized
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, git_merge_ai_resolver::resolution_engine::ResolutionError> {
        Ok(conflict.our_content.clone())
    }
}

fn main() {
    let mut results = Vec::new();
    let mut rng = rand::thread_rng();
    
    println!("Running git-merge-ai-resolver benchmarks...");
    println!("{:<15} {:<15} {:<15} {:<15}", "Conflicts", "Time (ms)", "Resolved", "Strategy");
    println!("{:-<60}", "");
    
    // Test with increasing numbers of conflicts
    for conflict_count in [1, 2, 5, 10, 20, 50] {
        // Generate a test file with the specified number of conflicts
        let conflict_file = generate_conflict_file(conflict_count, &mut rng);
        
        // Create a resolution engine with a whitespace-only strategy for benchmarking
        let mut engine = ResolutionEngine::new();
        engine.add_strategy(Box::new(WhitespaceOnlyStrategy::new()));
        
        // Measure resolution time
        let start = Instant::now();
        let result = engine.resolve_file(&conflict_file);
        let elapsed = start.elapsed();
        
        if let Ok(resolution) = result {
            let benchmark_result = BenchmarkResult {
                conflict_count,
                resolution_time: elapsed,
                resolved_conflicts: resolution.resolved_count,
                strategy_name: resolution.strategy_name,
            };
            
            println!(
                "{:<15} {:<15} {:<15} {:<15}",
                conflict_count,
                elapsed.as_millis(),
                resolution.resolved_count,
                resolution.strategy_name
            );
            
            results.push(benchmark_result);
        } else {
            println!("Error resolving conflicts: {:?}", result.err());
        }
    }
    
    // Summary
    println!("{:-<60}", "");
    println!("Benchmark complete!");
}