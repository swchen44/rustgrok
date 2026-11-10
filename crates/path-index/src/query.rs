// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
//! Bounded, deliberately incomplete single-field path expression syntax.
use crate::IndexError;
use rustgrok_analysis::analyze_path;

const MAX_TOKENS: usize = 128;
const MAX_DEPTH: usize = 16;

/// Occurrence of a child in the bounded OpenGrok-compatible path query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathOccur {
    Must,
    Should,
    MustNot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathClause {
    pub occur: PathOccur,
    pub query: PathQuery,
}

/// Engine-neutral tree; clause occurrence is retained even for unranked evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathQuery {
    Term(String),
    Boolean(Vec<PathClause>),
}

type Expr = PathQuery;
fn clause(occur: PathOccur, query: PathQuery) -> PathClause {
    PathClause { occur, query }
}
#[derive(Debug, PartialEq)]
enum Token<'a> {
    Word(&'a str),
    And,
    Or,
    Not,
    Plus,
    Minus,
    Open,
    Close,
}

fn lex(query: &str) -> Result<Vec<Token<'_>>, IndexError> {
    let bytes = query.as_bytes();
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        if bytes[at] == b' ' {
            at += 1;
            continue;
        }
        let token = match bytes[at] {
            b'(' => {
                at += 1;
                Token::Open
            }
            b')' => {
                at += 1;
                Token::Close
            }
            b'+' | b'-' => {
                let plus = bytes[at] == b'+';
                if at > 0 && bytes[at - 1] != b' ' && bytes[at - 1] != b'(' {
                    return Err(IndexError::UnsupportedQuery);
                }
                at += 1;
                if at == bytes.len() || !bytes[at].is_ascii_alphanumeric() {
                    return Err(IndexError::UnsupportedQuery);
                }
                if plus {
                    Token::Plus
                } else {
                    Token::Minus
                }
            }
            c if c.is_ascii_alphanumeric() || c == b'_' => {
                let start = at;
                while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_') {
                    at += 1;
                }
                let word = &query[start..at];
                match word {
                    "AND" => Token::And,
                    "OR" => Token::Or,
                    "NOT" => Token::Not,
                    _ => Token::Word(word),
                }
            }
            _ => return Err(IndexError::UnsupportedQuery),
        };
        out.push(token);
        if out.len() > MAX_TOKENS {
            return Err(IndexError::LimitExceeded);
        }
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    at: usize,
    analyze: fn(&str) -> Result<String, IndexError>,
}
impl Parser<'_> {
    fn atom(&mut self, depth: usize) -> Result<Expr, IndexError> {
        match self.tokens.get(self.at) {
            Some(Token::Word(word)) => {
                let term = (self.analyze)(word)?;
                self.at += 1;
                Ok(Expr::Term(term))
            }
            Some(Token::Open) => {
                if depth >= MAX_DEPTH {
                    return Err(IndexError::LimitExceeded);
                }
                self.at += 1;
                let result = self.group(depth + 1)?;
                if self.tokens.get(self.at) != Some(&Token::Close) {
                    return Err(IndexError::UnsupportedQuery);
                }
                self.at += 1;
                Ok(result)
            }
            _ => Err(IndexError::UnsupportedQuery),
        }
    }
    fn group(&mut self, depth: usize) -> Result<Expr, IndexError> {
        let negative = matches!(self.tokens.get(self.at), Some(Token::Not | Token::Minus));
        let plus = matches!(self.tokens.get(self.at), Some(Token::Plus));
        if negative || plus {
            self.at += 1;
        }
        let first = self.atom(depth)?;
        if negative {
            if !matches!(self.tokens.get(self.at), None | Some(Token::Close)) {
                return Err(IndexError::UnsupportedQuery);
            }
            return Ok(Expr::Boolean(vec![clause(PathOccur::MustNot, first)]));
        }
        let mut clauses = vec![clause(PathOccur::Must, first)];
        let mut has_excluded = false;
        let mut or_mode = false;
        let mut and_mode = false;
        loop {
            match self.tokens.get(self.at) {
                None | Some(Token::Close) => break,
                Some(Token::Or) => {
                    if and_mode || plus || has_excluded {
                        return Err(IndexError::UnsupportedQuery);
                    }
                    or_mode = true;
                    self.at += 1;
                    clauses.push(clause(PathOccur::Must, self.atom(depth)?));
                }
                Some(Token::And) => {
                    if or_mode {
                        return Err(IndexError::UnsupportedQuery);
                    }
                    and_mode = true;
                    self.at += 1;
                    clauses.push(clause(PathOccur::Must, self.atom(depth)?));
                }
                Some(Token::Not | Token::Minus) => {
                    if or_mode {
                        return Err(IndexError::UnsupportedQuery);
                    }
                    and_mode = true;
                    self.at += 1;
                    has_excluded = true;
                    clauses.push(clause(PathOccur::MustNot, self.atom(depth)?));
                }
                Some(Token::Word(_) | Token::Open) => {
                    if or_mode {
                        return Err(IndexError::UnsupportedQuery);
                    }
                    and_mode = true;
                    clauses.push(clause(PathOccur::Must, self.atom(depth)?));
                }
                _ => return Err(IndexError::UnsupportedQuery),
            }
        }
        if clauses.len() == 1 && !plus {
            return Ok(clauses.pop().ok_or(IndexError::UnsupportedQuery)?.query);
        }
        if or_mode {
            for clause in &mut clauses {
                clause.occur = PathOccur::Should;
            }
        }
        Ok(Expr::Boolean(clauses))
    }
}

pub fn parse_path_expression(query: &str, max_bytes: usize) -> Result<Expr, IndexError> {
    parse_expression(query, max_bytes, |word| {
        let parsed = analyze_path(word).map_err(|_| IndexError::UnsupportedQuery)?;
        if parsed.len() != 1 {
            return Err(IndexError::UnsupportedQuery);
        }
        Ok(parsed[0].term.clone())
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn parse_full_expression(query: &str, max_bytes: usize) -> Result<Expr, IndexError> {
    parse_expression(query, max_bytes, |word| {
        let parsed =
            rustgrok_analysis::analyze_full(word).map_err(|_| IndexError::UnsupportedQuery)?;
        if parsed.len() != 1 || parsed[0].start_byte != 0 || parsed[0].end_byte != word.len() {
            return Err(IndexError::UnsupportedQuery);
        }
        rustgrok_analysis::validate_full_index_terms(&parsed)
            .map_err(|_| IndexError::UnsupportedQuery)?;
        Ok(parsed[0].term.clone())
    })
}

fn parse_expression(
    query: &str,
    max_bytes: usize,
    analyze: fn(&str) -> Result<String, IndexError>,
) -> Result<Expr, IndexError> {
    if query.len() > max_bytes {
        return Err(IndexError::LimitExceeded);
    }
    let mut parser = Parser {
        tokens: lex(query)?,
        at: 0,
        analyze,
    };
    let result = parser.group(0)?;
    if parser.at != parser.tokens.len() {
        return Err(IndexError::UnsupportedQuery);
    }
    Ok(result)
}
