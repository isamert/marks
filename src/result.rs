use std::fmt;
use std::collections::HashMap;

use crate::args::Args;

#[derive(Debug, Clone)]
pub struct Header<'a> {
    /// Args
    pub args: &'a Args,
    /// On which line is the header found.
    pub line: usize,
    /// This usually means the count of # (for md) or * (for org) at the beginning of the header line.
    pub depth: usize,
    /// The header itself, stripped from tags or other annotations.
    pub content: String,
    /// Tags found in the header. Means nothing for markdown headers.
    pub tags: Vec<String>,
    /// Properties found in :PROPERTIES: block of an org header. Means nothing for markdown headers.
    pub properties: HashMap<String, String>,
}

impl<'a> Header<'a> {
    pub fn new(args: &Args) -> Header {
        Header {
            line: 0,
            depth: 0,
            content: String::new(),
            tags: vec![],
            properties: HashMap::new(),
            args,
        }
    }

    /// Concatenate given headers into single string with given seperator.
    pub fn concat(headers: &Vec<Header>, sep: &str) -> String {
        headers.iter().map(|x| x.content.to_owned()).collect::<Vec<_>>().join(sep)
    }
}

#[derive(Debug)]
pub struct SearchResult<'a> {
    /// Score.
    pub score: i64,
    /// Line number.
    pub line: usize,
    /// In which file?
    pub file_path: String,
    /// List of headers that this belongs to.
    pub headers: Vec<Header<'a>>,
    /// Full line content itself.
    pub content: String,
    pub args: &'a Args,
}

impl<'a> SearchResult<'a> {
    #[allow(unused_must_use)]
    pub fn print(&self) {
        if self.args.no_color {
            return println!("{}", self);
        }

        let mut t = term::stdout().unwrap();
        t.fg(term::color::MAGENTA).unwrap();
        write!(t, "{}", self.file_path).unwrap();

        t.fg(term::color::WHITE).unwrap();
        write!(t, ":").unwrap();

        t.fg(term::color::GREEN).unwrap();
        write!(t, "{}", self.line).unwrap();

        t.fg(term::color::WHITE).unwrap();
        write!(t, ":").unwrap();

        for header in self.headers.iter() {
            t.fg(term::color::BLUE).unwrap();
            write!(t, "{}", header.content).unwrap();
            t.fg(term::color::WHITE).unwrap();
            write!(t, "/").unwrap();
        }

        t.fg(term::color::WHITE).unwrap();
        if self.headers.len() > 0 {
            write!(t, ":").unwrap();
        }

        t.reset().unwrap();
        write!(t, "{}", self.content).unwrap();

        writeln!(t);
    }
}

/// Format SearchResult to print it out into
impl fmt::Display for SearchResult<'_> {
    #[allow(unused_must_use)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", &self.file_path, &self.line);
        if !self.args.no_headers {
            let headers = self.headers
                .iter()
                .map(|it| it.content.clone())
                .fold_first(|acc, it| format!("{}{}{}", acc, self.args.header_seperator, it))
                .unwrap_or(String::new());

            write!(f, ":{}", headers);
        }
        write!(f, ":{}", &self.content)
    }
}
