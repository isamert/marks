use std::fmt;

use crate::args::Args;

#[derive(Debug, Eq, PartialEq)]
pub struct SearchResult {
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
}

impl SearchResult {
    #[allow(unused_must_use)]
    pub fn print(&self, args: &Args) {
        let mut t = term::stdout().unwrap();

        if args.no_color {
            let file_and_line_sep: char = if args.null {
                '\0'
            } else {
                ':'
            };
            write!(t, "{}{}{}", &self.file_path, file_and_line_sep, &self.line);
            if !args.no_headers {
                let headers = self.headers.join(&args.header_seperator);
                write!(t, ":{}", headers);
            }
            write!(t, ":{}", &self.content);

            writeln!(t);
            return;
        }

        t.fg(term::color::MAGENTA).unwrap();
        write!(t, "{}", self.file_path).unwrap();

        if args.null {
            write!(t, "\0").unwrap();
        } else {
            t.fg(term::color::WHITE).unwrap();
            write!(t, ":").unwrap();
        };

        t.fg(term::color::GREEN).unwrap();
        write!(t, "{}", self.line).unwrap();

        t.fg(term::color::WHITE).unwrap();
        write!(t, ":").unwrap();

        let mut sep = "";
        for header in self.headers.iter() {
            t.fg(term::color::WHITE).unwrap();
            write!(t, "{}", sep).unwrap();

            sep = &args.header_seperator;

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
