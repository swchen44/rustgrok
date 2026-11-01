// SPDX-License-Identifier: MIT
// Independent G1 scorer probe. Tantivy defaults are not OpenGrok compatibility.
mod full_tokenizer;
use std::{env, fs, process};
use tantivy::{
    collector::{Count, TopDocs},
    doc,
    query::QueryParser,
    schema::{
        IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, STRING, TEXT,
    },
    Index, TantivyDocument,
};

fn arg(args: &[String], name: &str) -> Result<String, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .ok_or_else(|| format!("missing {name}"))
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(arg(&args, "--documents")?)?)?;
    let field_name = arg(&args, "--field")?;
    if !["full", "path", "def", "symbol"].contains(&field_name.as_str()) {
        return Err("unsupported field".into());
    }
    let query_text = arg(&args, "--query")?;
    if query_text.is_empty()
        || !query_text.chars().all(|c| {
            c.is_alphanumeric()
                || c == '_'
                || (field_name == "path" && (c == '.' || c == '/' || c == '-'))
        })
    {
        return Err("only one identifier or path query supported".into());
    }
    let projects: Vec<&str> = args
        .windows(2)
        .filter(|pair| pair[0] == "--project")
        .map(|pair| pair[1].as_str())
        .collect();
    let selected: Vec<_> = input
        .iter()
        .filter(|d| {
            projects.is_empty() || d["project"].as_str().is_some_and(|p| projects.contains(&p))
        })
        .collect();
    if selected.len() > 10_000 {
        return Err("probe limited to 10000 documents".into());
    }
    if field_name == "full" {
        full_tokenizer::validate(&query_text)?;
        for document in &selected {
            full_tokenizer::validate(document["content"].as_str().ok_or("missing content")?)?;
        }
    }
    let mut builder = Schema::builder();
    let path = builder.add_text_field("path", STRING | STORED);
    let options = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("opengrok_full_g1")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let text = if field_name == "full" {
        builder.add_text_field("text", options)
    } else {
        builder.add_text_field("text", TEXT)
    };
    let index = Index::create_in_ram(builder.build());
    index
        .tokenizers()
        .register("opengrok_full_g1", full_tokenizer::FullTokenizer::default());
    let mut writer = index.writer_with_num_threads(1, 50_000_000)?;
    for document in &selected {
        let path_str = document["path"].as_str().ok_or("missing path")?;
        let value = match field_name.as_str() {
            "path" => path_str.to_owned(),
            "def" => document["definitions"]
                .as_array()
                .ok_or("missing definitions")?
                .iter()
                .filter_map(|v| v["name"].as_str())
                .collect::<Vec<_>>()
                .join(" "),
            _ => document["content"]
                .as_str()
                .ok_or("missing content")?
                .to_owned(),
        };
        writer.add_document(doc!(path => path_str, text => value))?;
    }
    writer.commit()?;
    let reader = index.reader()?;
    reader.reload()?;
    let searcher = reader.searcher();
    let mut parser = QueryParser::for_index(&index, vec![text]);
    parser.set_conjunction_by_default();
    let query = parser.parse_query(&query_text)?;
    let (count, top) = searcher.search(
        &query,
        &(
            Count,
            TopDocs::with_limit(selected.len().max(1)).order_by_score(),
        ),
    )?;
    let mut hits = Vec::new();
    for (score, address) in top {
        let document: TantivyDocument = searcher.doc(address)?;
        let file = document
            .get_first(path)
            .and_then(|v| v.as_str())
            .ok_or("missing stored path")?;
        let explain: serde_json::Value =
            serde_json::from_str(&query.explain(&searcher, address)?.to_pretty_json())?;
        hits.push(serde_json::json!({"path":file,"score":score,"score_bits":format!("{:08x}",score.to_bits()),"segment_ord":address.segment_ord,"doc_id":address.doc_id,"explain":explain}));
    }
    println!(
        "{}",
        serde_json::json!({"role":"experimental Tantivy defaults; not OpenGrok parity","indexed_documents":selected.len(),"resultCount":count,"hits":hits})
    );
    Ok(())
}
fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(2)
    }
}
