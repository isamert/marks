use regex::Regex;

use combine::parser::char::{char, spaces};
use combine::stream::easy::ParseError;
use combine::{between, choice, many1, satisfy, sep_by, EasyParser, Parser};

#[derive(Debug)]
pub enum QueryToken {
    Regex(Regex),
    Must(String),
    None(String),
    Plain(String),
}

#[derive(Debug)]
pub struct Query {
    /// Query string that user provided.
    pub full: String,
    /// "keyword"
    pub musts: Vec<String>,
    /// -keyword
    pub nones: Vec<String>,
    /// `(some|regex)`
    pub regexes: Vec<Regex>,
    /// full - (musts + nones + regexes). Used for fuzzy searching.
    pub rest: Vec<String>,
}

/// Solely for testing
impl PartialEq for Query {
    fn eq(&self, other: &Self) -> bool {
        self.full == other.full
            && self.musts == other.musts
            && self.nones == other.nones
            && self.regexes.iter().zip(other.regexes.iter()).all(|(x, y)| x.as_str() == y.as_str())
            && self.rest == other.rest
    }
}

impl Eq for Query {}

impl Query {
    pub fn new(input: &str) -> Result<Query, ParseError<&str>> {
        let full = input.to_string();
        let mut musts: Vec<String> = vec![];
        let mut nones = vec![];
        let mut regexes = vec![];
        let mut rest = vec![];

        let non_ws = satisfy(|x| x != ' ');
        let non_quote = satisfy(|x| x != '"');
        let non_backtick = satisfy(|x| x != '`');

        let token = choice((
            between(char('"'), char('"'), many1(non_quote)).map(|x: String| QueryToken::Must(x)),
            between(char('`'), char('`'), many1(non_backtick))
                .map(|x: String| QueryToken::Regex(Regex::new(&x).unwrap())),
            (char('-'), many1(non_ws)).map(|x| QueryToken::None(x.1)),
            many1(non_ws).map(|x| QueryToken::Plain(x)),
        ));
        let mut query = sep_by(token, spaces());
        let result: Result<(Vec<QueryToken>, &str), ParseError<&str>> = query.easy_parse(input);

        result?.0.into_iter().for_each(|x| match x {
            QueryToken::Regex(r) => regexes.push(r),
            QueryToken::Plain(r) => rest.push(r),
            QueryToken::Must(r) => musts.push(r),
            QueryToken::None(r) => nones.push(r),
        });

        Ok(Query {
            full,
            musts,
            nones,
            regexes,
            rest,
        })
    }
}

impl Default for Query {
    fn default() -> Self {
        Query {
            full: String::new(),
            musts: vec![],
            nones: vec![],
            regexes: vec![],
            rest: vec![],
        }
    }
}

#[test]
fn test_parse_query() {
    assert_eq!(Query::new("").unwrap(), Query::default());
    assert_eq!(Query::new("-badword \"stuff\" \"another stuff\" hehe `a regex`").unwrap(), Query {
        full: "-badword \"stuff\" \"another stuff\" hehe `a regex`".into(),
        musts: vec!["stuff".into(), "another stuff".into()],
        nones: vec!["badword".into()],
        rest: vec!["hehe".into()],
        regexes: vec![Regex::new("a regex").unwrap()],
        ..Default::default()
    });
}
