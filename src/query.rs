//! Query parser
//! This module contains the query parser, used to parse search queries.
//! The query syntax is similar to Google's search syntax, but with some
//! modifications to make it more suitable for file searching.
//!
//! Tokens are separated by whitespace, except for quoted strings
//! which are treated as a single token.
//! If a token starts with a -, it is treated as an exclusion.
//! Else, it is treated as an inclusion.
//! a normal string is regarded as a fuzzy match
//! If a token contains a :, it is treated as a key:value operation
//! for example, `prefix:foo` will match all files that start with `foo`
//!

//
// Path: src/query.rs

use std::{collections::BTreeMap, path::PathBuf};
use futures_core::stream::Stream;

use async_stream::stream;
use fuzzy_matcher::FuzzyMatcher;
// let's use nom to parse the query, and skim to do the fuzzy matching
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    IResult,
};

use color_eyre::Result;

use crate::files::{Index, IndexedFile};

/// A query term
/// All terms will be parsed as a NormalFuzzy term, unless they start with a reserved keyword, followed by a colon
/// Which turns them into an operation.
/// for example, `prefix:foo` will match all files that start with `foo`
/// `mime:application/pdf` will match all files that have the MIME type `application/pdf`
/// `tag:foo` will match all files that have the tag `foo`
/// `regex:/foo/` will match all files that contain `foo`
/// and so on
/// If a term starts with a -, it is treated as an exclusion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    /// Match by fuzzy search
    NormalFuzzy(String),
    /// Match by the prefix of the file name, including the extension
    Prefix(String),
    /// Match by the suffix of the file name, including the extension
    Suffix(String),
    /// Match by the suffix of the file name, without the extension
    SuffixName(String),
    /// Match by file extension
    Extension(String),
    /// Matches by MIME type
    Mime(String),
    /// Matches by tag
    Tag(String),
    /// Match by exact string
    Exact(String),
    /// Regex match
    /// Regex format will be the same as the one used by ripgrep, but in between slashes
    /// For example, `/foo/` will match all files that contain `foo`
    /// `/foo/i` will match all files that contain `foo` case-insensitively
    Regex(String),
    /// Modified before
    /// Matches all files that were modified before the given date
    Before(String),
    /// Modified after
    /// Matches all files that were modified after the given date
    After(String),
}

impl Term {
    pub fn match_rules(&self, file: &IndexedFile) -> bool {
        match self {
            Term::NormalFuzzy(_) => {
                // we have already done the fuzzy matching in the query parser
                true
            }
            Term::Regex(s) => {
                let name = file.path.file_name().unwrap().to_str().unwrap();
                let re = regex::Regex::new(s).unwrap();
                re.is_match(name)
            }
            Term::Exact(s) => {
                let name = file.path.file_name().unwrap().to_str().unwrap();
                name.contains(s)
            }
            Term::Prefix(s) => file
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap()
                .starts_with(s),
            Term::Suffix(s) => file
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap()
                .ends_with(s),
            Term::SuffixName(s) => {
                let name = {
                    if let Some(name) = file.path.file_stem() {
                        name.to_str().unwrap().split('.').next().unwrap()
                    } else {
                        file.path.file_name().unwrap().to_str().unwrap()
                    }
                };
                name.ends_with(s)
            }
            Term::Extension(s) => {
                if let Some(ext) = file.path.extension() {
                    ext.to_str().unwrap() == s
                } else {
                    false
                }
            }
            Term::Mime(s) => file.data_type == Some(s.clone()),
            Term::Tag(s) => file.tags.contains(s),
            _ => todo!(),
        }
    }
}

// our control group
#[test]
fn test_query() {
    let query = "prefix:foo suffix:bar baz -qux -\"aaa bbb\" -extension:md #owo -#uwu";

    let query = parse_query(query).unwrap();

    println!("{:#?}", query);
    assert_eq!(
        query,
        Query {
            includes: vec![
                Term::Prefix(String::from("foo")),
                Term::Suffix(String::from("bar")),
                Term::NormalFuzzy(String::from("baz")),
                Term::Tag(String::from("owo")),
            ],
            excludes: vec![
                Term::Exact(String::from("qux")),
                Term::Exact(String::from("aaa bbb")),
                Term::Extension(String::from("md")),
                Term::Tag(String::from("uwu")),
            ],
        }
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub includes: Vec<Term>,
    pub excludes: Vec<Term>,
}

// TODO: dedup this please

fn parse_prefix(input: &str) -> IResult<&str, Term> {
    // dont skip whitespace or anything that is escaped
    let (input, _) = alt((
        tag("prefix:"),
        tag("pre:"),
        tag("start:"),
        tag("starts_with:"),
        tag("pfx:"),
    ))(input)?;
    let (input, prefix) = take_while1(|_| true)(input)?;

    Ok((input, Term::Prefix(String::from(prefix))))
}

fn parse_suffix(input: &str) -> IResult<&str, Term> {
    let (input, _) = alt((
        tag("suffix:"),
        tag("suf:"),
        tag("end:"),
        tag("ends_with:"),
        tag("sfx:"),
    ))(input)?;
    let (input, suffix) = take_while1(|_| true)(input)?;

    Ok((input, Term::Suffix(String::from(suffix))))
}

fn parse_suffix_name(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("suffix_name:")(input)?;
    let (input, suffix) = take_while1(|_| true)(input)?;

    Ok((input, Term::SuffixName(String::from(suffix))))
}

fn parse_extension(input: &str) -> IResult<&str, Term> {
    let (input, _) = alt((tag("extension:"), tag("ext:"), tag("file:")))(input)?;
    let (input, extension) = take_while1(|_| true)(input)?;

    Ok((input, Term::Extension(String::from(extension))))
}

fn parse_mime(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("mime:")(input)?;
    let (input, mime) = take_while1(|c: char| c.is_ascii() || c == '_')(input)?;

    Ok((input, Term::Mime(String::from(mime))))
}

fn parse_tag(input: &str) -> IResult<&str, Term> {
    let (input, _) = alt((tag("#"), tag("tag:"), tag("tags:"), tag("tagged:")))(input)?;
    let (input, tag) = take_while1(|_| true)(input)?;

    Ok((input, Term::Tag(String::from(tag))))
}

fn parse_exact(input: &str) -> IResult<&str, Term> {
    let (input, _) = alt((tag("@"), tag("exact:")))(input)?;
    let (input, exact) = take_while1(|_| true)(input)?;

    Ok((input, Term::Exact(String::from(exact))))
}

fn parse_regex(input: &str) -> IResult<&str, Term> {
    let (input, _) = alt((
        tag("regex:"),
        tag("re:"),
        tag("r:"),
        tag("regexp:"),
        tag("rgx:"),
    ))(input)?;
    let (input, regex) = take_while1(|_| true)(input)?;

    Ok((input, Term::Regex(String::from(regex))))
}

fn parse_before(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("before:")(input)?;
    let (input, before) = take_while1(|_| true)(input)?;

    Ok((input, Term::Before(String::from(before))))
}

fn parse_after(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("after:")(input)?;
    let (input, after) = take_while1(|_| true)(input)?;

    Ok((input, Term::After(String::from(after))))
}

fn parse_fuzzy(input: &str) -> IResult<&str, Term> {
    // do nothing and just return the input
    Ok((input, Term::NormalFuzzy(String::from(input))))
}

/// This function is used to parse a single term from a query.
/// It will take any string and return a Term enum.
fn parse_term(input: &str) -> IResult<&str, Term> {
    let (input, term) = alt((
        parse_regex,
        parse_prefix,
        parse_extension,
        parse_suffix_name,
        parse_suffix,
        parse_before,
        parse_after,
        parse_mime,
        parse_tag,
        parse_exact,
        parse_fuzzy,
    ))(input)?;

    Ok((input, term))
}

/// This function breaks down a query into a list of tokens,
/// then looks for a special negation token `-` and then
/// turns all the tokens into a Query struct.
/// The terms will be sorted by whether they are negated or not.
pub fn parse_query(query: &str) -> Result<Query> {
    let mut includes = Vec::new();
    let mut excludes = Vec::new();

    // Separate the query into tokens, unless escaped or quoted like posix
    // if contains, ", then turn it into a single token until the next "
    // if contains \, then escape the next character
    // else, split by whitespace

    let mut query = query;

    let mut buf = Vec::new();
    let mut token_buf = String::new();
    while let Some(ch) = query.chars().next() {
        // println!("ch: {}", ch);
        match ch {
            '\\' => {
                query = &query[1..];
                token_buf.push(query.chars().next().unwrap());
            }
            '"' => {
                query = &query[1..];
                while let Some(ch) = query.chars().next() {
                    if ch == '"' {
                        // only break this loop, not the outer one
                        break;
                    }
                    token_buf.push(ch);
                    query = &query[1..];
                }
            }
            ' ' => {
                buf.push(token_buf);
                // println!("space hit");
                // println!("buf: {:#?}", buf);
                token_buf = String::new();
            }
            _ => {
                // println!("pushing {}", ch);
                token_buf.push(ch);
            }
        }
        // advance the query
        query = &query[1..];
    }

    // if there's anything left in the token buffer, push it
    if !token_buf.is_empty() {
        buf.push(token_buf);
    }

    // println!("{:#?}", buf);

    // parse the tokens
    for token in buf {
        if token.starts_with('-') {
            let term = if let Some(term) = token.strip_prefix('-') {
                parse_term(term).unwrap().1
            } else {
                parse_term(&token).unwrap().1
            };

            // if term is fuzzyterm, turn it into exact
            let term = match term {
                Term::NormalFuzzy(term) => Term::Exact(term),
                _ => term,
            };
            excludes.push(term);
        } else {
            let (_, term) = parse_term(&token).unwrap();
            includes.push(term);
        }
    }

    Ok(Query { includes, excludes })
}

/// Make a fuzzy match score depending on the Query

pub fn fuzzy_score(query: &Query, ixf: IndexedFile) -> Result<(i64, IndexedFile)> {
    let includes = query
        .includes
        .iter()
        .filter_map(|term| match term {
            Term::NormalFuzzy(term) => Some(term),
            _ => None,
        })
        .collect::<Vec<_>>();

    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default()
        .smart_case()
        .use_cache(true)
        .debug(false);

    let mut score: i64 = 0;

    for term in includes {
        let mut max_score = 0;
        let path = ixf.path.clone();
        if let Some(score) = matcher.fuzzy_match(path.to_str().unwrap(), term) {
            max_score = score;
        }
        score += max_score;
    }

    Ok((score, ixf))
}

// New fuzzy match to score each entry individually
/// Evaluate the score of the file based on the query
pub fn eval_score(query: &Query, ixf: IndexedFile) -> Result<Option<(i64, IndexedFile)>> {
    // do fuzzy score first
    let (score, file) = fuzzy_score(query, ixf.clone())?;

    // then do the filters
    let mut cond = false;

    {
        for term in &query.includes {
            if term.match_rules(&file) {
                cond = true;
            } else {
                // If it doesn't match the rules once, it should fail
                // cond = false;
                return Ok(None);
            }
        }
    }
    // if in any case it fails, we should return false
    for term in &query.excludes {
        if term.match_rules(&file) {
            // cond = false;
            return Ok(None);
        }
    }

    if cond {
        Ok(Some((score, file)))
    } else {
        Ok(None)
    }
}

// let's use skim to match the queries

/// This function does the actual fuzzy matching of the query.
/// If there are no fuzzy terms, it will return everything.
pub fn fuzzy_match(query: &Query, idx: &Index) -> Vec<(i64, IndexedFile)> {
    // println!("Loaded index: {:#?}", file);
    // get the NormalFuzzy terms
    let (includes, _excludes): (Vec<_>, Vec<_>) = query
        .includes
        .iter()
        .filter_map(|term| match term {
            Term::NormalFuzzy(term) => Some(term),
            _ => None,
        })
        .partition(|term| {
            !query
                .excludes
                .contains(&Term::NormalFuzzy(term.to_string()))
        });

    // println!("includes: {:#?}", includes);

    // turn the index into a vec of strings

    if includes.is_empty() {
        // return everything
        return idx.files.iter().map(|f| (0, f.clone())).collect();
    }

    // return a sorted list of matches, excluding the ones that do not match
    let mut matches = BTreeMap::new();

    // use fuzzy_score to score the matches
    for file in idx.files.iter() {
        let (score, _) = fuzzy_score(query, file.clone()).unwrap();
        matches.insert(file.to_owned(), score);
    }

    // sort matches by score
    let mut matches = matches.into_iter().map(|(k, v)| (v, k)).collect::<Vec<_>>();
    matches.sort_by(|a, b| b.0.cmp(&a.0));
    // println!("{:#?}", matches);
    matches
}

// The actual query function
/// This is the main entrypoint for querying the index.
/// It will first try to fuzzy match the query, them finally
/// filters them by the rules provided in the Term enum.
pub fn query(query: &Query, index: &Index) -> Vec<(i64, IndexedFile)> {
    // first, let's try to match the query with fuzzy matching

    let mut scored_index = index
        .files
        .iter()
        .map(|f| eval_score(query, f.to_owned()))
        .filter(|f| f.is_ok())
        .filter(|f| f.as_ref().unwrap().is_some())
        .map(|f| f.unwrap().unwrap())
        .collect::<Vec<_>>();

    // sort matches by score
    scored_index.sort_by(|a, b| b.0.cmp(&a.0));
    // reverse the order
    scored_index.reverse();
    scored_index
}


// Query, but stream the results instead of collecting them
/// Streaming version of the query function.
pub fn query_stream(query: Query, index: Index) -> impl Stream<Item = (i64, IndexedFile)> {
    // first, let's try to match the query with fuzzy matching
    let s = stream! {
        for file in index.files.iter() {
            if let Ok(Some((score, file))) = eval_score(&query, file.to_owned()) {
                yield (score, file);
            }
        }
    };
    s
}