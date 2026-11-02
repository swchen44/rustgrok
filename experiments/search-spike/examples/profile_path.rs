// SPDX-License-Identifier: MIT
// Instrumentation experiment; not a product profiler or a compatibility claim.
#[allow(dead_code)]
#[path = "../src/path_response.rs"]
mod path_response;
use serde::{Deserialize, Serialize};
use std::{env, fs, time::Instant};

#[allow(dead_code)]
#[derive(Deserialize)]
struct Document {
    path: String,
    project: String,
    content: String,
    definitions: Vec<Definition>,
}
#[allow(dead_code)]
#[derive(Deserialize)]
struct Definition {
    name: String,
    line: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Hit {
    path: String,
    line_numbers: Vec<usize>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Results {
    result_count: usize,
    results: Vec<Hit>,
}

fn arg(args: &[String], name: &str) -> Result<String, String> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
        .ok_or_else(|| format!("missing {name}"))
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let index = arg(&args, "--index")?;
    let query = arg(&args, "--query")?;
    if query.is_empty()
        || !query
            .chars()
            .all(|c| c.is_alphanumeric() || "_./-".contains(c))
    {
        return Err("only one identifier query is supported".into());
    }
    let projects: Vec<&str> = args
        .windows(2)
        .filter(|w| w[0] == "--project")
        .map(|w| w[1].as_str())
        .collect();
    let count: usize = arg(&args, "--samples")?.parse()?;
    if !(1..=1000).contains(&count) {
        return Err("samples must be in 1..=1000".into());
    }
    let mut samples = Vec::new();
    let mut expected = None;
    let mut observed_bytes = 0;
    let mut observed_documents = 0;
    let mut observed_selected = 0;
    for _ in 0..count {
        let started = Instant::now();
        let bytes = fs::read(&index)?;
        let read_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        let docs: Vec<Document> = serde_json::from_slice(&bytes)?;
        let parse_ns = started.elapsed().as_nanos();
        let documents = docs.len();
        let started = Instant::now();
        let mut tokenize_ns = 0u128;
        let mut selected = 0usize;
        let mut hits = Vec::new();
        for d in docs {
            if !projects.is_empty() && !projects.contains(&d.project.as_str()) {
                continue;
            }
            selected += 1;
            let _lowercase = query.to_lowercase();
            let token_start = Instant::now();
            let matches = path_response::path_matches(&d.path, &query);
            tokenize_ns += token_start.elapsed().as_nanos();
            if matches {
                hits.push(Hit {
                    path: d.path,
                    line_numbers: vec![],
                });
            }
        }
        let scan_total_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        let result = serde_json::to_string(&Results {
            result_count: hits.len(),
            results: hits,
        })?;
        let serialize_ns = started.elapsed().as_nanos();
        if let Some(previous) = expected.as_ref() {
            if previous != &result
                || observed_bytes != bytes.len()
                || observed_documents != documents
                || observed_selected != selected
            {
                return Err("input or result changed during profiling".into());
            }
        } else {
            expected = Some(result);
            observed_bytes = bytes.len();
            observed_documents = documents;
            observed_selected = selected;
        }
        samples.push(serde_json::json!({"read_ns":read_ns,"parse_ns":parse_ns,"scan_total_ns":scan_total_ns,"tokenize_ns":tokenize_ns,"serialize_ns":serialize_ns}));
    }
    println!(
        "{}",
        serde_json::json!({"scope":"instrumented CLI path search; no HTTP timing, no physical I/O attribution","index_bytes":observed_bytes,"documents":observed_documents,"selected_documents":observed_selected,"result":serde_json::from_str::<serde_json::Value>(expected.as_deref().ok_or("missing result")?)?,"samples":samples})
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
