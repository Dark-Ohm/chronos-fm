use super::backend::SearchBackend;
use super::SearchResult;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

pub struct SpotlightBackend;

impl SpotlightBackend {
    pub fn new() -> Self {
        Self
    }

    fn grep_in_file(&self, path: &PathBuf, query: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return results,
        };
        let reader = BufReader::new(file);
        let query_lower = query.to_lowercase();

        // Simple case-insensitive search
        for (index, line) in reader.lines().enumerate() {
            if let Ok(content) = line {
                if content.len() > 1000 {
                    continue; // Skip very long lines
                }

                if content.to_lowercase().contains(&query_lower) {
                    results.push(SearchResult {
                        path: path.clone(),
                        line_number: index + 1,
                        line_content: content.trim().to_string(),
                    });

                    // Sanity limit per file to avoid OOM on massive matches, but increased significantly
                    if results.len() >= 1000 {
                        break;
                    }
                }
            }
        }

        results
    }
}

impl SearchBackend for SpotlightBackend {
    fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Use mdfind to locate files
        // -0 for null delimiter is safer but string split matching output lines is easier for simple impl
        // mdfind outputs one path per line.
        let output = Command::new("mdfind")
            .arg(query)
            .output()
            .context("Failed to execute mdfind")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("mdfind failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        // Removed MAX_TOTAL_RESULTS to return all results as requested
        for line in stdout.lines() {
            let path = PathBuf::from(line);
            if path.exists() && path.is_file() {
                let file_matches = self.grep_in_file(&path, query);
                results.extend(file_matches);
            }
        }

        Ok(results)
    }
}
