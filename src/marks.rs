use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use walkdir::DirEntry;
use std::collections::HashMap;
use std::iter::Peekable;
use chrono::prelude::*;
use std::io;
use rayon::prelude::*;


use combine::Stream;
use combine::{satisfy, choice,  between, many, many1, sep_by, sep_end_by1, Parser, EasyParser, any, token, tokens, count, optional, count_min_max};
use combine::parser::char::{spaces, char, alpha_num, digit, letter};
use combine::error::{ParseError};
use {
    combine::{
        error::{Commit},
        parser::{
            function::parser,
        },
        stream::{
            buffered,
            position::{self, SourcePosition},
            IteratorStream,
        },
        StreamOnce,
    },
};

use crate::args::Args;
use crate::query::Query;
use crate::utils::file_utils;
use crate::result::{Header, SearchResult};

lazy_static! {
    // TODO: make it extendable
    static ref BLACKLIST: Vec<&'static str> = vec!["node_modules"];
}

pub struct Marks<'a> {
    pub args: &'a Args,
    pub query: Query,
    pub matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum DocType {
    Markdown,
    OrgMode,
}

#[derive(Debug)]
pub enum OrgDatePlan {
    /// SCHEDULED dates
    Scheduled,
    /// DEADLINE dates
    Deadline,
    /// Just plain dates, no DEADLINE or SCHEDULED prefix
    Plain,
}

/// Some possible formats:
/// <2003-09-16 Tue 12:00-12:30>
#[derive(Debug)]
pub struct OrgDateTime {
    /// <...> is for active dates, [...] is for passive dates.
    pub is_active: bool,
    /// Is it SCHEDULED, DEADLINE or just plain date?
    pub date_plan: OrgDatePlan,
    /// First date found in the org datetime.
    pub date_start: DateTime<Utc>,
    /// Second date found in the org datetime. Following formats has the second date:
    /// <...>--<...>
    /// <... HH:MM-HH-MM>.
    pub date_end: Option<DateTime<Utc>>,
    /// Invertal. Not quite useful at this point.
    /// https://orgmode.org/manual/Repeated-tasks.html
    pub invertal: Option<String>,
}

// TODO: move to utils
pub fn starts_with_ignore_case(l: &str, r: &str) -> bool {
    l.get(..r.len()).map(|x| x.eq_ignore_ascii_case(r)).unwrap_or(false)
}

/// Additional mutation methods for `Option`.
pub trait StartsWithIgnoreCase {
    fn starts_with_i(&self, pre: &str) -> bool;
}

impl StartsWithIgnoreCase for String {
    fn starts_with_i(&self, other: &str) -> bool {
        self.get(..other.len()).map(|x| x.eq_ignore_ascii_case(other)).unwrap_or(false)
    }
}

/// Parse `HH:MM`.
fn hour<Input>() -> impl Parser<Input, Output = (u32, u32)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        token(':'),
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap())
    ).map(|(h, _, m)| (h, m))
}

/// Parse `HH:MM-HH:MM`. Second part is optional.
fn hour_range<Input>() -> impl Parser<Input, Output = ((u32, u32), Option<(u32, u32)>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        hour(),
        optional(token('-').and(hour()))
    ).map(|(x, y)| (x, y.map(|(_, a)| a)))
}

impl<'a> Marks<'a> {
    pub fn new(args: &'a Args, query: Query) -> Marks {
        // TODO: parametrize this
        let matcher = SkimMatcherV2::default();

        Marks {
            query,
            args,
            matcher,
        }
    }

    pub fn find_files(&'a self) -> impl Iterator<Item = DirEntry> + 'a {
        WalkDir::new(&self.args.path)
            .into_iter()
            .filter_entry(move |e| !file_utils::is_hidden(e) && !self.is_file_blacklisted(e))
            .filter_map(|e| e.ok())
            .filter_map(move |e| {
                if e.file_type().is_file() {
                    if (!self.args.no_org && self.is_org_file(&e)) || (!self.args.no_markdown && self.is_md_file(&e)) {
                        return Some(e)
                    }
                }

                return None
            })
    }

    pub fn search_file(&self, file: &DirEntry) -> Option<Vec<SearchResult>> {
        let filename = file.file_name().to_str()?;
        let doc_type = self.get_doc_type(&file);

        let reader = BufReader::new(File::open(file.path()).ok()?);
        let mut results = vec![];

        let mut headers: Vec<Header> = vec![];
        let mut last_depth = 0;

        let mut iter = reader.lines().filter_map(|x| x.ok()).enumerate().peekable();
        while let Some((index, line)) = iter.next() {
            let header_info = self.parse_header(&mut iter, &doc_type, &line);
            let is_header = header_info.is_some();

            if let Some((depth, header_content)) = header_info {
                let header = Header {
                    depth,
                    content: header_content,
                    line: index + 1,
                    properties: HashMap::new(),
                    tags: vec![],
                    args: self.args,
                };

                if depth > last_depth {
                    headers.push(header);
                } else if last_depth == depth {
                    let lastn = headers.len() - 1;
                    headers[lastn] = header;
                } else {
                    headers.truncate(depth);

                    // (depth - 1) will not work because header hiearchy may go like this:
                    // * ***
                    let curr_len = headers.len();
                    headers[curr_len - 1] = header;
                }
                last_depth = depth;
            }

            // TODO: Maybe don't do this every loop?
            let mut full: String = headers
                .iter()
                .map(|x| x.content.to_owned())
                .collect::<Vec<_>>()
                .join(" / ");

            if !is_header {
                full.push_str(&line);
            }
            if self.args.search_filename {
                full.push_str(&filename);
            }


            // Check regexes
            if !self.query.regexes.iter().all(|x| x.is_match(&full)) {
                continue
            }

            // Check musts
            if !self.query.musts.iter().all(|x| full.contains(x)) {
                continue
            }

            // Check nones
            if self.query.nones.iter().any(|x| full.contains(x)) {
                continue
            }

            // Fuzzy match
            let points = self.query.rest.iter().filter_map(|q| self.matcher.fuzzy_match(&full, &q)).collect::<Vec<_>>();
            if points.len() > 0 || self.query.rest.len() == 0 {
                results.push(SearchResult {
                    line: index + 1,
                    file_path: file.path().to_str()?.to_string(),
                    score: points.iter().sum::<i64>(),
                    headers: headers.to_vec(),
                    content: line,
                    args: self.args,
                });
            }
        }

        return Some(results);
    }

    fn is_file_blacklisted(&'a self, entry: &DirEntry) -> bool {
        entry.file_name()
            .to_str()
            .map(|s| BLACKLIST.contains(&s))
            .unwrap_or(false)
    }

    fn is_org_file(&'a self, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.args.org_extension.iter().any(|y| x == y),
            None => false
        }
    }

    fn is_md_file(&'a self, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.args.md_extension.iter().any(|y| x == y),
            None => false
        }
    }

    fn get_doc_type(&self, file: &DirEntry) -> DocType {
        if self.is_md_file(file) {
            return DocType::Markdown;
        } else {
            return DocType::OrgMode;
        }
    }

    fn parse_header<I>(&self, iter: &mut Peekable<I>, typ: &DocType, line: &str) -> Option<(usize, String)>
    where I: Iterator<Item = (usize, String)> {
        let x = match typ {
            DocType::Markdown => '#',
            DocType::OrgMode => '*',
        };

        let mut chars = line.chars().into_iter();
        if chars.next() != Some(x) {
            return None;
        }

        let mut depth: usize = 1;
        for chr in &mut chars {
            if chr == x {
                depth += 1;
                continue;
            } else if chr == ' ' {
                break;
            } else {
                return None;
            }
        }

        // TODO: parse_tags, parse_props, parse_header_date and put them in Header struct then return it
        //       it might be good if user does not search for these, simply don't parse them
        //       ex. if --prop does not exist in args, simply skip parse_props() call
        let (tags, rest) = self.parse_tags(&mut chars);
        let datetime = self.parse_header_date(iter);
        if datetime.is_some() {
            println!("{:?}", datetime);
        }

        return Some((depth, rest));
    }

    fn parse_header_date<I>(&self, iter: &mut Peekable<I>) -> Option<OrgDateTime>
    where I: Iterator<Item = (usize, String)> {
        // Only ISO 8601 dates are supported
        // TODO: handle plain timestamps after headers
        let has_schedule = iter
            .peek()
            .map(|(_, x)| x.starts_with_i("DEADLINE:") || x.starts_with_i("SCHEDULED:"))
            .unwrap_or(false);

        if has_schedule {
            let (_, line_date) = iter.next().unwrap();

            let invertal_parser = many1(satisfy(|x| x != '>' && x != ']'));
            let mut date_parser = (
                spaces().silent(),
                many1(letter()).map(|x: String| match x.as_str() {
                    "DEADLINE" => OrgDatePlan::Deadline,
                    "SCHEDULED" => OrgDatePlan::Scheduled,
                    _ => OrgDatePlan::Plain, // FIXME: this is wrong, it should not happen
                }),
                token(':'),
                spaces().silent(),
                choice((token('<'), token('['))).map(|c| c == '<'), // < means active, [ means inactive
                count(4, digit()).map(|x: String| x.parse::<i32>().unwrap()),
                token('-'),
                count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
                token('-'),
                count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
                spaces(),
                count(3, letter()).map(|x: String| x),
                spaces().silent(),
                optional(hour_range()).map(|hour| hour.unwrap_or(((0, 0), None))),
                spaces().silent(),
                optional(invertal_parser),
                choice((token(']'), token('>'))),
            ).map(|(_, date_plan, _, _, is_active, year, _, month, _, day, _, _day_str, _, hour, _, invertal, _)| OrgDateTime {
                is_active,
                date_plan,
                date_start: Utc.ymd(year, month, day).and_hms(hour.0.0, hour.0.1, 0),
                date_end: hour.1.map(|end| Utc.ymd(year, month, day).and_hms(end.0, end.1, 0)),
                invertal,
            });

            let result: Result<(OrgDateTime, &str), _> = date_parser.easy_parse("DEADLINE: <2020-12-18 Fri 18:30-20:30>");
            result.ok().map(|x| x.0)
        } else {
            None
        }
    }

    /// Parse the tags from given line and return the tags along with the header that is stripped from the tags and whitespace.
    fn parse_tags<I>(&self, chars: &mut I) -> (Vec<String>, String)
    where I: DoubleEndedIterator<Item = char> {
        // TODO: https://orgmode.org/guide/Tags.html
        //       According to here tags should be inherited by child headers, this does not support this.
        //       This can be handled while doing the search.
        let mut rev_chars = chars.rev().peekable();
        let has_tags = rev_chars.peek().map(|x| *x == ':').unwrap_or(false);
        let rev_header = rev_chars.collect::<String>();

       if has_tags {
            let mut tags_parser = (
                spaces().silent(),
                token(':'),
                sep_end_by1(many1(alpha_num()), token(':')).map(|xs: Vec<String>| xs.iter().map(|x| x.chars().rev().collect()).collect()),
                spaces().silent(),
            ).map(|(_, _, tags, _)| tags);

            let result: Result<(Vec<String>, &str), _> = tags_parser.parse(rev_header.as_str());
            if let Ok((tags, rest)) = result {
                (tags, rest.chars().rev().collect::<String>())
            } else {
                (vec![], chars.collect())
            }
        } else {
            (vec![], chars.collect())
        }
    }

    fn parse_props<I>(&self, iter: &mut Peekable<I>) -> HashMap<String, String>
    where I: Iterator<Item = (usize, String)> {
        let has_props = iter.peek().map(|(_, x)| x.starts_with(":PROPERTIES:")).unwrap_or(false);
        if has_props {
            let mut props: HashMap<String, String> = HashMap::new();
            iter.next(); // Consume :PROPERTIES:

            while let Some((_, prop)) = iter.next() {
                if prop.starts_with(":END:") {
                    return props
                } else {
                    let non_colon = satisfy(|x| x != ':');
                    let mut prop_parser = (
                        between(char(':'), char(':'), many1(non_colon)),
                        spaces(),
                        many(any())
                    ).map(|(key, _, val)| (key, val));

                    let result: Result<((String, String), &str), _> = prop_parser.parse(&prop);
                    if let Ok(((key, val), _)) = result {
                        props.insert(key, val);
                    } else {
                        println!("{:?}", result);
                    }
                }
            }
        }

        HashMap::new()
    }
}
