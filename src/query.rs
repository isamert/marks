use regex::Regex;
use std::iter::Peekable;

use super::parser;

#[derive(Debug)]
pub enum QueryToken {
    Regex(Regex),
    Must(String),
    None(String),
    Plain(String),
}

#[derive(Debug)]
pub struct Query {
    pub full: String,
    // "keyword"
    pub musts: Vec<String>,
    // -keyword
    pub nones: Vec<String>,
    // TODO regex:
    pub regexes: Vec<Regex>,
    // splitted fuzzy searches
    pub rest: Vec<String>,
}

impl Query {
    pub fn new(input: &str) -> Query {
        let full = input.to_string();
        let mut musts = vec![];
        let mut nones = vec![];
        let mut regexes = vec![];
        let mut rest = vec![];

        let iter = &mut full.chars().peekable();
        while let Some(x) = Query::tokenize_single(iter) {
            match x {
                QueryToken::Regex(r) => regexes.push(r),
                QueryToken::Plain(r) => rest.push(r),
                QueryToken::Must(r) => musts.push(r),
                QueryToken::None(r) => nones.push(r),
            }
        }

        Query { full, musts, nones, regexes, rest }
    }

    pub fn tokenize_single<I>(iter: &mut Peekable<I>) -> Option<QueryToken>
    where I: Iterator<Item = char> {
        while parser::parse_whitespace(iter) {
            continue
        }

        parser::parse_around(iter, '"', '"').map(QueryToken::Must)
            .or_else(|| parser::parse_around(iter, '`', '`').map(|x| QueryToken::Regex(Regex::new(&x).unwrap())))
            .or_else(|| parser::parse_prefixed(iter, '-').map(QueryToken::None))
            .or_else(|| parser::parse_plain(iter).map(QueryToken::Plain))
    }
}
