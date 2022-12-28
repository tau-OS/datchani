// Datchani Search Query Parser
// Tokens are separated by whitespace, except for quoted strings
// which are treated as a single token.
// If a token starts with a -, it is treated as an exclusion.
// Else, it is treated as an inclusion.
// a normal string is regarded as a fuzzy match
// If a token contains a :, it is treated as a key:value operation
// for example, `prefix:foo` will match all files that start with `foo`
//

//
// Path: src/query.rs

use std::{collections::BTreeMap, path::PathBuf};

use fuzzy_matcher::FuzzyMatcher;
// let's use nom to parse the query, and skim to do the fuzzy matching
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    IResult,
};

use color_eyre::Result;

use crate::files::{Index, IndexedFile};

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
}

impl Term {
    pub fn match_rules(&self, file: &IndexedFile) -> bool {
        match self {
            Term::NormalFuzzy(_) => {
                // we have already done the fuzzy matching in the query parser
                true
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
            Term::Tag(s) => {
                file.tags.contains(s)
            }
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
    let (input, _) = tag("prefix:")(input)?;
    let (input, prefix) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::Prefix(String::from(prefix))))
}

fn parse_suffix(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("suffix:")(input)?;
    let (input, suffix) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::Suffix(String::from(suffix))))
}

fn parse_suffix_name(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("suffix_name:")(input)?;
    let (input, suffix) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::SuffixName(String::from(suffix))))
}

fn parse_extension(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("extension:")(input)?;
    let (input, extension) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::Extension(String::from(extension))))
}

fn parse_mime(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("mime:")(input)?;
    let (input, mime) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::Mime(String::from(mime))))
}

fn parse_tag(input: &str) -> IResult<&str, Term> {
    let (input, _) = tag("#")(input)?;
    let (input, tag) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    Ok((input, Term::Tag(String::from(tag))))
}

fn parse_fuzzy(input: &str) -> IResult<&str, Term> {
    // do nothing and just return the input
    Ok((input, Term::NormalFuzzy(String::from(input))))
}

fn parse_term(input: &str) -> IResult<&str, Term> {
    let (input, term) = alt((
        parse_prefix,
        parse_suffix,
        parse_suffix_name,
        parse_extension,
        parse_mime,
        parse_tag,
        parse_fuzzy,
    ))(input)?;

    Ok((input, term))
}

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

// let's use skim to match the queries

pub fn fuzzy_match(query: &Query, idx: &Index) -> Vec<(i64, String)> {
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
    let index = idx
        .files
        .iter()
        .map(|file| file.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    if includes.is_empty() {
        // return everything
        return index
            .iter()
            .map(|path| (0, path.to_owned()))
            .collect::<Vec<_>>();
    }

    // nah lets try fuzzy-matcher
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default()
        .smart_case()
        .debug(false);

    // return a sorted list of matches, excluding the ones that do not match
    let mut matches = BTreeMap::new();

    for term in includes {
        for item in &index {
            let mat = matcher.fuzzy_match(item, term);
            if let Some(score) = mat {
                matches
                    .entry(item)
                    .and_modify(|v| *v += score)
                    .or_insert(score);
            }
        }
    }

    // sort matches by score
    let mut matches = matches
        .into_iter()
        .map(|(k, v)| (v, k.to_owned()))
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| b.0.cmp(&a.0));
    // println!("{:#?}", matches);
    matches
}

// The actual query function
pub fn query(query: &Query, index: &Index) -> Vec<(i64, IndexedFile)> {
    // first, let's try to match the query with fuzzy matching
    let matches = fuzzy_match(query, index);
    // println!("{:#?}", matches);

    // now we can map it into the actual Index struct

    // sort the matches by score
    // matches.sort_by(|a, b| b.0.cmp(&a.0));
    matches
        .into_iter()
        .map(|(score, path)| (score, index.get_file(PathBuf::from(path)).unwrap()))
        .filter(|(_, file)| {
            // filter out the rules
            let mut cond = false;
            {
                for term in &query.includes {
                    // ignore if it's a fuzzy term
                    if let Term::NormalFuzzy(_) = term {
                        continue;
                    }
                    if term.match_rules(file) {
                        cond = true;
                        break;
                    } else {
                        // If it doesn't match the rules once, it should fail
                        cond = false;
                        return cond;
                    }
                }
            }

            for term in &query.excludes {
                if term.match_rules(file) {
                    cond = false;
                    break;
                }
            }

            cond
        })
        .map(|(score, file)| (score, file.clone()))
        .collect::<Vec<_>>()
}
