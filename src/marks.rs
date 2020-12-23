use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use walkdir::DirEntry;
use std::collections::HashMap;
use std::iter::Peekable;
use combine::Parser;

use crate::args::Args;
use crate::utils::file_utils;
use crate::result::SearchResult;
use crate::extensions::StartsWithIgnoreCase;
use crate::org::datetime::OrgDateTime;
use crate::org::header::OrgHeader;
use crate::parsers;

lazy_static! {
    // TODO: make it extendable
    static ref BLACKLIST: Vec<&'static str> = vec!["node_modules"];
}

pub struct Marks<'a> {
    pub args: &'a Args,
    pub matcher: SkimMatcherV2,
}

#[derive(Debug, Clone)]
pub enum DocType {
    Markdown,
    OrgMode,
}

impl<'a> Marks<'a> {
    pub fn new(args: &'a Args) -> Marks {
        // TODO: parametrize this
        let matcher = SkimMatcherV2::default();

        Marks {
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

        let mut headers: Vec<OrgHeader> = vec![];
        let mut last_depth = 0;
        let mut skip_section = false;

        let mut iter = reader.lines().filter_map(|x| x.ok()).enumerate().peekable();
        while let Some((index, line)) = iter.next() {
            let header_info = self.parse_header(&mut iter, &doc_type, &line, index);
            let is_header = header_info.is_some();

            if let Some(header) = header_info {
                let depth = header.depth;

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

                // Check if any of the headers in the hierarchy contains the given tags
                // or the given props. Skip the check if we already found match in any of the parent headers.
                if !(!skip_section && depth > last_depth) {
                    let matches_tags = self.args.tagged
                        .iter()
                        .all(|x| headers.iter().any(|header| header.tags.contains(x)));

                    let matches_props = self.args.prop
                        .iter()
                        .all(|(key, val)| headers
                             .iter()
                             .any(|header| header
                                  .properties
                                  .get(key)
                                  .map(|header_val| header_val == val)
                                  .unwrap_or(false)));

                    skip_section = !(matches_tags && matches_props)
                }
            }

            // Skip 0-level if are looking for props or tags
            // FIXME: For level-0 we might want to parse  #+TITLE #+FILETAGS etc. to make the check
            //        but this requires these constructs to be found at the top of the file, otherwise
            //        they'll become pointless.
            if last_depth == 0 && (!self.args.tagged.is_empty() || !self.args.prop.is_empty()) {
                skip_section = true;
            }

            if skip_section {
                continue;
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
            if !self.args.query.regexes.iter().all(|x| x.is_match(&full)) {
                continue
            }

            // Check musts
            if !self.args.query.musts.iter().all(|x| full.contains(x)) {
                continue
            }

            // Check nones
            if self.args.query.nones.iter().any(|x| full.contains(x)) {
                continue
            }

            // Fuzzy match
            let points = self.args.query.rest.iter().filter_map(|q| self.matcher.fuzzy_match(&full, &q)).collect::<Vec<_>>();
            if points.len() > 0 || self.args.query.rest.len() == 0 {
                results.push(SearchResult {
                    line: index + 1,
                    file_path: file.path().to_str()?.to_string(),
                    score: points.iter().sum::<i64>(),
                    headers: headers.iter().map(|x| x.content.to_string()).collect(),
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

    fn parse_header<I>(&self, iter: &mut Peekable<I>, typ: &DocType, line: &str, idx: usize) -> Option<OrgHeader>
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

        // TODO: it might be good if user does not search for these, simply don't parse them
        //       ex. if --prop does not exist in args, simply skip parse_org_props() call etc.
        let (tags, content) = self.parse_org_tags(&mut chars);
        // FIXME: properties may come after datetime or vice versa. Not really sure tho
        let datetime = self.parse_org_date_time(iter);
        let properties = self.parse_org_props(iter);

        Some(OrgHeader {
            depth,
            content,
            properties,
            tags,
            datetime,
            line: idx,
            args: self.args,
        })
    }

    fn parse_org_date_time<I>(&self, iter: &mut Peekable<I>) -> Option<OrgDateTime>
    where I: Iterator<Item = (usize, String)> {
        // Only ISO 8601 dates are supported
        // TODO: handle plain timestamps after headers
        let has_schedule = iter
            .peek()
            .map(|(_, x)| x.starts_with_i("DEADLINE:") || x.starts_with_i("SCHEDULED:"))
            .unwrap_or(false);

        if has_schedule {
            let (_, line_date) = iter.next().unwrap();
            let result: Result<(OrgDateTime, &str), _> = parsers::org_date_time().parse(line_date.as_str());
            result.ok().map(|x| x.0)
        } else {
            None
        }
    }

    /// Parse the tags from given line and return the tags along with the header that is stripped from the tags and whitespace.
    fn parse_org_tags<I>(&self, chars: &mut I) -> (Vec<String>, String)
    where I: DoubleEndedIterator<Item = char> {
        // TODO: https://orgmode.org/guide/Tags.html
        //       According to here tags should be inherited by child headers, this does not support this.
        //       This can be handled while doing the search.
        let mut rev_chars = chars.rev().peekable();
        let has_tags = rev_chars.peek().map(|x| *x == ':').unwrap_or(false);
        let rev_header = rev_chars.collect::<String>();

       if has_tags {
            let result: Result<(Vec<String>, &str), _> = parsers::org_tags().parse(rev_header.as_str());
            if let Ok((tags, rest)) = result {
                (tags, rest.chars().rev().collect::<String>())
            } else {
                (vec![], rev_header.chars().rev().collect())
            }
        } else {
            (vec![], rev_header.chars().rev().collect())
        }
    }

    fn parse_org_props<I>(&self, iter: &mut Peekable<I>) -> HashMap<String, String>
    where I: Iterator<Item = (usize, String)> {
        let has_props = iter.peek().map(|(_, x)| x.starts_with_i(":PROPERTIES:")).unwrap_or(false);
        let mut props: HashMap<String, String> = HashMap::new();

        if has_props {
            iter.next(); // Consume :PROPERTIES:

            while let Some((_, prop)) = iter.next() {
                if prop.starts_with_i(":END:") {
                    return props
                } else {
                    let result: Result<((String, String), &str), _> = parsers::org_property().parse(&prop);
                    if let Ok(((key, val), _)) = result {
                        props.insert(key, val);
                    } else {
                        // Probably :PROPERTIES: block does not have :END:
                        // (I just assumed this to not to consume whole file, it might just be a bad property line)
                        // so just return what we just have found so far
                        return props
                    }
                }
            }
        }

        props
    }
}
