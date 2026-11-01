// SPDX-License-Identifier: MIT
// Independently authored experimental code; not a port of OpenGrok source.
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

#[derive(Serialize, Deserialize)]
struct Document {
    path: String,
    project: String,
    content: String,
    definitions: Vec<Definition>,
}
#[derive(Serialize, Deserialize)]
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
fn visit(
    root: &Path,
    path: &Path,
    ctags: &Path,
    documents: &mut Vec<Document>,
) -> Result<(), String> {
    for item in fs::read_dir(path).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let p = item.path();
        let kind = item.file_type().map_err(|e| e.to_string())?;
        if item.file_name() == ".git" {
            continue;
        }
        if kind.is_symlink() {
            return Err(format!("symlink unsupported: {}", p.display()));
        }
        if kind.is_dir() {
            visit(root, &p, ctags, documents)?;
        } else if kind.is_file() {
            let relative = p.strip_prefix(root).map_err(|e| e.to_string())?;
            let components: Vec<_> = relative
                .components()
                .map(|x| x.as_os_str().to_string_lossy().into_owned())
                .collect();
            if components.len() < 2 {
                continue;
            }
            let content = fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let out = process::Command::new(ctags)
                .args([
                    "--options=NONE",
                    "--output-format=json",
                    "--sort=no",
                    "--fields=+n",
                    "--extras=-F",
                    "-f",
                    "-",
                ])
                .arg(&p)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!(
                    "ctags failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            let mut definitions = Vec::new();
            for line in out.stdout.split(|b| *b == b'\n').filter(|s| !s.is_empty()) {
                let v: serde_json::Value =
                    serde_json::from_slice(line).map_err(|e| e.to_string())?;
                if v["_type"] == "tag" {
                    if let (Some(name), Some(number)) = (v["name"].as_str(), v["line"].as_u64()) {
                        definitions.push(Definition {
                            name: name.into(),
                            line: number as usize,
                        });
                    }
                }
            }
            documents.push(Document {
                path: format!("/{}", components.join("/")),
                project: components[0].clone(),
                content,
                definitions,
            });
        }
    }
    Ok(())
}
fn search(index: &Path, args: &[String]) -> Result<String, String> {
    let field = arg(args, "--field")?;
    let query = arg(args, "--query")?;
    if !["full", "path", "def", "symbol"].contains(&field.as_str()) {
        return Err(format!("unsupported experimental field: {field}"));
    }
    if query.is_empty()
        || !query.chars().all(|c| {
            c.is_alphanumeric()
                || c == '_'
                || (field == "path" && (c == '.' || c == '/' || c == '-'))
        })
    {
        return Err("only one identifier query is supported".into());
    }
    let projects: Vec<&str> = args
        .windows(2)
        .filter(|w| w[0] == "--project")
        .map(|w| w[1].as_str())
        .collect();
    let docs: Vec<Document> = serde_json::from_slice(&fs::read(index).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut hits = Vec::new();
    for d in docs {
        if !projects.is_empty() && !projects.contains(&d.project.as_str()) {
            continue;
        }
        let q = query.to_lowercase();
        let lines: Vec<usize> = match field.as_str() {
            "path" => {
                if d.path.to_lowercase().contains(&q) {
                    vec![]
                } else {
                    continue;
                }
            }
            "def" => d
                .definitions
                .iter()
                .filter(|v| v.name == query)
                .map(|v| v.line)
                .collect(),
            "symbol" => d
                .content
                .lines()
                .enumerate()
                .filter(|(_, line)| {
                    line.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|word| word == query)
                })
                .map(|(i, _)| i + 1)
                .collect(),
            _ => d
                .content
                .lines()
                .enumerate()
                .filter(|(_, line)| {
                    line.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|word| word.to_lowercase() == q)
                })
                .map(|(i, _)| i + 1)
                .collect(),
        };
        if field == "path" || !lines.is_empty() {
            hits.push(Hit {
                path: d.path,
                line_numbers: lines,
            });
        }
    }
    serde_json::to_string(&Results {
        result_count: hits.len(),
        results: hits,
    })
    .map_err(|e| e.to_string())
}
fn decode(value: &str) -> Result<String, String> {
    let mut output = Vec::new();
    let mut it = value.bytes();
    while let Some(b) = it.next() {
        match b {
            b'+' => output.push(b' '),
            b'%' => {
                let hi = it.next().ok_or("invalid percent escape")?;
                let lo = it.next().ok_or("invalid percent escape")?;
                let hex = |b: u8| (b as char).to_digit(16).ok_or("invalid percent escape");
                output.push((hex(hi)? * 16 + hex(lo)?) as u8);
            }
            _ => output.push(b),
        }
    }
    String::from_utf8(output).map_err(|e| e.to_string())
}
fn serve(index: &Path, address: &str) -> Result<(), String> {
    use std::io::{BufRead, BufReader, Read, Write};
    let endpoint: std::net::SocketAddr = address.parse().map_err(|_| "invalid listen address")?;
    if !endpoint.ip().is_loopback() {
        return Err("experimental server accepts loopback only".into());
    }
    let listener = std::net::TcpListener::bind(endpoint).map_err(|e| e.to_string())?;
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let mut line = String::new();
        let result: Result<String, String> = (|| {
            BufReader::new(&stream)
                .take(8192)
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            let mut parts = line.split_whitespace();
            if parts.next() != Some("GET") {
                return Err("only GET supported".into());
            }
            let target = parts.next().ok_or("missing request target")?;
            let raw = target
                .strip_prefix("/probe/search?")
                .ok_or("unknown experimental endpoint")?;
            let mut args = Vec::new();
            for pair in raw.split('&') {
                let (key, value) = pair.split_once('=').ok_or("invalid query")?;
                let key = decode(key)?;
                if !["field", "query", "project"].contains(&key.as_str()) {
                    return Err("unknown parameter".into());
                }
                args.push(format!("--{key}"));
                args.push(decode(value)?);
            }
            search(index, &args)
        })();
        let (status, body) = match result {
            Ok(body) => ("200 OK", body),
            Err(error) => (
                "400 Bad Request",
                serde_json::json!({"error":error}).to_string(),
            ),
        };
        write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).map_err(|e|e.to_string())?;
    }
    Ok(())
}
fn run(args: &[String]) -> Result<(), String> {
    let action = args.first().ok_or("expected index or search")?;
    let index = PathBuf::from(arg(args, "--index")?);
    match action.as_str() {
        "index" => {
            let root = fs::canonicalize(arg(args, "--source")?).map_err(|e| e.to_string())?;
            if index.exists() {
                return Err("index already exists".into());
            }
            let ctags = PathBuf::from(arg(args, "--ctags")?);
            let name = index.file_name().ok_or("invalid index filename")?;
            let parent = index
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let resolved_index = fs::canonicalize(parent)
                .map_err(|e| e.to_string())?
                .join(name);
            if resolved_index.starts_with(&root) {
                return Err("index must be outside source root".into());
            }
            let mut docs = Vec::new();
            visit(&root, &root, &ctags, &mut docs)?;
            let bytes = serde_json::to_vec(&docs).map_err(|e| e.to_string())?;
            use std::io::Write;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(resolved_index)
                .and_then(|mut f| f.write_all(&bytes))
                .map_err(|e| e.to_string())?;
        }
        "search" => println!("{}", search(&index, args)?),
        "serve" => serve(&index, &arg(args, "--listen")?)?,
        _ => return Err("expected index or search".into()),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run(&env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("{error}");
        process::exit(2);
    }
}
