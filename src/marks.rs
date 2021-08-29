use combine::Parser;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::Peekable;
use walkdir::DirEntry;
use walkdir::WalkDir;

use crate::args::Args;
use crate::extensions::StartsWithIgnoreCase;
use crate::org::datetime::OrgDateTime;
use crate::org::header::OrgHeader;
use crate::parsers;
use crate::result::SearchResult;
use crate::utils::file_utils;

pub struct Marks<'a> {
    pub args: &'a Args,
    pub matcher: SkimMatcherV2,
}

#[derive(Debug)]
pub struct Searcher<T: Iterator>
where T::Item: std::fmt::Debug {
    pub typ: DocType,
    pub results: Vec<SearchResult>,
    pub headers: Vec<OrgHeader>,
    pub last_depth: usize,
    pub skip_section: bool,
    pub iter: Peekable<T>,
    pub iter_current: (usize, String)
}

impl<T: Iterator> Searcher<T>
where T::Item: std::fmt::Debug,
      T: Iterator<Item = (usize, String)>,
{
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

        Marks { args, matcher }
    }

    pub fn find_files(&'a self) -> impl Iterator<Item = DirEntry> + 'a {
        WalkDir::new(&self.args.path)
            .into_iter()
            .filter_entry(move |e| !file_utils::is_hidden(e) && !self.is_file_blacklisted(e))
            .filter_map(|e| e.ok())
            .filter_map(move |e| {
                if e.file_type().is_file() {
                    if (!self.args.no_org && self.is_org_file(&e))
                        || (!self.args.no_markdown && self.is_md_file(&e))
                    {
                        return Some(e);
                    }
                }

                return None;
            })
    }

    // TODO: refactor/divide into smaller functions
    pub fn search_file(&self, file: &DirEntry) -> Option<Vec<SearchResult>> {
        let filename = file.file_name().to_str()?;
        let doc_type = self.get_doc_type(&file);

        let reader = BufReader::new(File::open(file.path()).ok()?);
        let mut searcher = Searcher {
            typ: doc_type,
            results: vec![],
            headers: vec![],
            last_depth: 0,
            skip_section: false,
            iter: reader.lines()
                .filter_map(|x| x.ok())
                .enumerate()
                .peekable(),
            iter_current: (0, String::new()),
        };

        while let Some(iter_current) = searcher.iter.next() {
            searcher.iter_current = iter_current;
            //let header_info = org_header();
            let header_info: Option<OrgHeader> = None;
            let is_header = header_info.is_some();

            if let Some(header) = header_info {
                let depth = header.depth;

                if depth > searcher.last_depth {
                    searcher.headers.push(header);
                } else if searcher.last_depth == depth {
                    let lastn = searcher.headers.len() - 1;
                    searcher.headers[lastn] = header;
                } else {
                    searcher.headers.truncate(depth);

                    // (depth - 1) will not work because header hiearchy may go like this:
                    // * ***
                    let curr_len = searcher.headers.len();
                    searcher.headers[curr_len - 1] = header;
                }
                searcher.last_depth = depth;

                // Check if any of the searcher.headers in the hierarchy contains the given tags
                // or the given props. Skip the check if we already found match in any of the parent searcher.headers.
                if !(!searcher.skip_section && depth > searcher.last_depth) {
                    let matches_tags = self
                        .args
                        .tagged
                        .iter()
                        .all(|x| searcher.headers.iter().any(|header| header.tags.contains(x)));

                    let matches_props = self.args.prop.iter().all(|(key, val)| {
                        searcher.headers.iter().any(|header| {
                            header
                                .properties
                                .get(key)
                                .map(|header_val| header_val == val)
                                .unwrap_or(false)
                        })
                    });

                    searcher.skip_section = !(matches_tags && matches_props)
                }

                if !searcher.skip_section {
                    let curr_header = searcher.headers.last().unwrap();
                    if !self.args.todo.is_empty() {
                        let has_todo = self
                            .args
                            .todo
                            .iter()
                            .any(|x| curr_header.todo.as_ref().map_or(false, |y| y == x));
                        searcher.skip_section = searcher.skip_section || !has_todo;
                    }

                    if !self.args.priority.is_empty() {
                        let is_right_priority = self
                            .args
                            .priority
                            .iter()
                            .any(|x| curr_header.priority.as_ref().map_or(false, |y| x == y));

                        searcher.skip_section = searcher.skip_section || !is_right_priority;
                    }

                    if let Some(priority) = &self.args.priority_lt {
                        let is_lt_than = curr_header
                            .priority
                            .as_ref()
                            .map_or(false, |x| x < priority);
                        searcher.skip_section = searcher.skip_section || !is_lt_than;
                    }

                    if let Some(priority) = &self.args.priority_gt {
                        let is_gt_than = curr_header
                            .priority
                            .as_ref()
                            .map_or(false, |x| x > priority);
                        searcher.skip_section = searcher.skip_section || !is_gt_than;
                    }

                    if let Some(schedule) = &self.args.scheduled_at {
                        //println!("{:?}", curr_header.datetime);
                        searcher.skip_section = searcher.skip_section || !curr_header
                            .datetime
                            .as_ref()
                            .map_or(false, |datetime| datetime.compare_with(schedule, PartialEq::eq, PartialEq::eq));
                    }
                }
            }

            // Skip 0-level if are looking for props or tags
            // FIXME: For level-0 we might want to parse  #+TITLE #+FILETAGS etc. to make the check
            //        but this requires these constructs to be found at the top of the file, otherwise
            //        they'll become pointless.
            if searcher.last_depth == 0
                && (!self.args.tagged.is_empty()
                    || !self.args.prop.is_empty()
                    || !self.args.priority.is_empty()
                    || self.args.priority_lt.is_some()
                    || self.args.priority_gt.is_some()
                    || self.args.scheduled_at.is_some())
            {
                searcher.skip_section = true;
            }

            if searcher.skip_section {
                continue;
            }

            // TODO: Maybe don't do this every loop?
            let full: String = {
                let mut result = searcher.headers
                    .iter()
                    .map(|x| x.content.to_owned())
                    .collect::<Vec<_>>()
                    .join(" / ");

                if !is_header {
                    // TODO: implement
                    // result.push_str(&line);
                }

                if self.args.search_filename {
                    result.push_str(&filename);
                }

                result
            };

            // Check regexes
            if !self.args.query.regexes.iter().all(|x| x.is_match(&full))
                || !self.args.query.musts.iter().all(|x| full.contains(x))
                || self.args.query.nones.iter().any(|x| full.contains(x))
            {
                continue;
            }

            // Fuzzy match
            let points = self
                .args
                .query
                .rest
                .iter()
                .filter_map(|q| self.matcher.fuzzy_match(&full, &q))
                .collect::<Vec<_>>();
            if points.len() > 0 || self.args.query.rest.len() == 0 {
                searcher.results.push(SearchResult {
                    // TODO: implement
                    line:  1,
                    // line: index + 1,
                    file_path: file.path().to_str()?.to_string(),
                    score: points.iter().sum::<i64>(),
                    headers: searcher.headers.iter().map(|x| x.content.to_string()).collect(),
                    // TODO: implement
                    // content: line,
                    content: String::new()
                });
            }
        }

        return Some(searcher.results);
    }

    fn is_file_blacklisted(&'a self, entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| self.args.blacklist_folder.contains(&s.to_string()))
            .unwrap_or(false)
    }

    fn is_org_file(&'a self, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.args.org_extension.iter().any(|y| x == y),
            None => false,
        }
    }

    fn is_md_file(&'a self, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.args.md_extension.iter().any(|y| x == y),
            None => false,
        }
    }

    fn get_doc_type(&self, file: &DirEntry) -> DocType {
        if self.is_md_file(file) {
            return DocType::Markdown;
        } else {
            return DocType::OrgMode;
        }
    }


    fn parse_org_date_time<I>(&self, iter: &mut Peekable<I>) -> Option<OrgDateTime>
    where
        I: Iterator<Item = (usize, String)>,
    {
        // Only ISO 8601 dates are supported
        // TODO: handle plain timestamps after headers
        let has_schedule = iter
            .peek()
            .map(|(_, x)| x.starts_with_i("DEADLINE:") || x.starts_with_i("SCHEDULED:"))
            .unwrap_or(false);

        if has_schedule {
            let (_, line_date) = iter.next().unwrap();
            let result: Result<(OrgDateTime, &str), _> =
                parsers::org_date_time().parse(line_date.as_str());
            result.ok().map(|x| x.0)
        } else {
            None
        }
    }

    fn parse_org_props<I>(&self, iter: &mut Peekable<I>) -> HashMap<String, String>
    where
        I: Iterator<Item = (usize, String)>,
    {
        let has_props = iter
            .peek()
            .map(|(_, x)| x.starts_with_i(":PROPERTIES:"))
            .unwrap_or(false);
        let mut props: HashMap<String, String> = HashMap::new();

        if has_props {
            iter.next(); // Consume :PROPERTIES:

            while let Some((_, prop)) = iter.next() {
                if prop.starts_with_i(":END:") {
                    return props;
                } else {
                    let result: Result<((String, String), &str), _> =
                        parsers::org_property().parse(&prop);
                    if let Ok(((key, val), _)) = result {
                        props.insert(key, val);
                    } else {
                        // Probably :PROPERTIES: block does not have :END:
                        // (I just assumed this to not to consume whole file, it might just be a bad property line)
                        // so just return what we just have found so far
                        return props;
                    }
                }
            }
        }

        props
    }
}
