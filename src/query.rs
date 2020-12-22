use regex::Regex;

use combine::{satisfy, choice,  between, many1, sep_by, Parser, EasyParser};
use combine::parser::char::{spaces, char};
use combine::stream::easy::ParseError;

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
            between(char('`'), char('`'), many1(non_backtick)).map(|x: String| QueryToken::Regex(Regex::new(&x).unwrap())),
            (char('-'), many1(non_ws)).map(|x| QueryToken::None(x.1)),
            many1(non_ws).map(|x| QueryToken::Plain(x)),
        ));
        let mut query = sep_by(token, spaces());
        let result: Result<(Vec<QueryToken>, &str), ParseError<&str>> = query.easy_parse(input);

        result?.0.into_iter().for_each(|x| {
            match x {
                QueryToken::Regex(r) => regexes.push(r),
                QueryToken::Plain(r) => rest.push(r),
                QueryToken::Must(r) => musts.push(r),
                QueryToken::None(r) => nones.push(r),
            }
        });

        Ok(Query { full, musts, nones, regexes, rest })
    }
}
