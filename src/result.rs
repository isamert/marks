use std::fmt;
use std::collections::HashMap;

use crate::org::header::OrgHeader;
use crate::args::Args;

#[derive(Debug)]
pub struct SearchResult<'a> {
    /// Score.
    pub score: i64,
    /// Line number.
    pub line: usize,
    /// In which file?
    pub file_path: String,
    /// List of headers that this belongs to.
    pub headers: Vec<String>,
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

        let mut sep = "";
        for header in self.headers.iter() {
            t.fg(term::color::WHITE).unwrap();
            write!(t, "{}", sep).unwrap();

            sep = &self.args.header_seperator;

            t.fg(term::color::BLUE).unwrap();
            write!(t, "{}", header).unwrap();
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
            let headers = self.headers.join(&self.args.header_seperator);

            write!(f, ":{}", headers);
        }
        write!(f, ":{}", &self.content)
    }
}
