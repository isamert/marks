use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    iter::Peekable,
};

use combine::Parser;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use walkdir::DirEntry;

use crate::{
    args::Args,
    extensions::StartsWithIgnoreCase,
    org::{datetime::OrgDateTime, header::OrgHeader},
    parsers,
    result::SearchResult,
};


pub struct Searcher<'a, T: Iterator>
where
    T::Item: std::fmt::Debug,
{
    pub args: &'a Args,
    pub filename: &'a str,
    pub filepath: &'a str,
    pub headers: Vec<OrgHeader>,
    pub last_depth: usize,
    pub skip_section: bool,
    pub iter: Peekable<T>,
    pub iter_current: (usize, String),
    pub matcher: SkimMatcherV2,
}


pub fn new_searcher<'a>(
    args: &'a Args,
    file: &'a DirEntry,
) -> Option<Searcher<'a, impl Iterator<Item = (usize, String)>>> {
    let reader = BufReader::new(File::open(file.path()).ok()?);
    let iter = reader
        .lines()
        .filter_map(|x| x.ok())
        .enumerate()
        .peekable();
    Some(Searcher {
        args,
        filename: file.file_name().to_str()?,
        filepath: file.path().to_str()?,
        matcher: SkimMatcherV2::default(),
        headers: vec![],
        last_depth: 0,
        skip_section: false,
        iter,
        iter_current: (0, String::new()),
    })
}


impl<'a, T: Iterator> Searcher<'a, T>
where
    T::Item: std::fmt::Debug,
    T: Iterator<Item = (usize, String)>,
{
    pub fn search(&mut self) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = vec![];

        while let Some(iter_current) = self.iter.next() {
            self.iter_current = iter_current;
            let header_info = self.parse_header();
            let is_header = header_info.is_some();

            self.handle_header(header_info);
            self.skip_section = self.should_skip_section();

            if self.skip_section {
                continue;
            }

            let full = self.build_current_result_line(is_header);

            if !self.query_matches(&full) {
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
                results.push(SearchResult {
                    line: self.iter_current.0 + 1,
                    file_path: self.filepath.to_string(),
                    score: points.iter().sum::<i64>(),
                    headers: self.headers.iter().map(|x| x.content.to_string()).collect(),
                    content: self.current_line().to_string(),
                });
            }
        }

        results
    }

    fn query_matches(&self, full: &str) -> bool {
        self.args.query.regexes.iter().all(|x| x.is_match(full))
            && self.args.query.musts.iter().all(|x| full.contains(x))
            && !self.args.query.nones.iter().any(|x| full.contains(x))
    }

    fn build_current_result_line(&self, is_current_header: bool) -> String {
        let mut result = self
            .headers
            .iter()
            .map(|x| x.content.to_owned())
            .collect::<Vec<_>>()
            .join(" / ");

        if !is_current_header {
            result.push_str(self.current_line());
        }

        if self.args.search_filename {
            result.push_str(self.filename);
        }

        result
    }

    fn should_skip_section(&self) -> bool {
        // Skip 0-level if are looking for props or tags
        // FIXME: For level-0 we might want to parse  #+TITLE #+FILETAGS etc. to make the check
        //        but this requires these constructs to be found at the top of the file, otherwise
        //        they'll become pointless.
        self.last_depth == 0
            && (!self.args.tagged.is_empty()
                || !self.args.prop.is_empty()
                || !self.args.priority.is_empty()
                || self.args.priority_lt.is_some()
                || self.args.priority_gt.is_some()
                || self.args.scheduled_at.is_some())
    }

    fn handle_header(&mut self, header_info: Option<OrgHeader>) {
        if let Some(header) = header_info {
            let depth = header.depth;

            if depth > self.last_depth {
                self.headers.push(header);
            } else if self.last_depth == depth {
                let lastn = self.headers.len() - 1;
                self.headers[lastn] = header;
            } else {
                self.headers.truncate(depth);

                // (depth - 1) will not work because header hiearchy may go like this:
                // * ***
                let curr_len = self.headers.len();
                self.headers[curr_len - 1] = header;
            }
            self.last_depth = depth;

            // Check if any of the self.headers in the hierarchy
            // contains the given tags or the given props. Skip the
            // check if we already found match in any of the parent
            // self.headers.
            if !(!self.skip_section && depth > self.last_depth) {
                let matches_tags = self
                    .args
                    .tagged
                    .iter()
                    .all(|x| self.headers.iter().any(|header| header.tags.contains(x)));

                let matches_props = self.args.prop.iter().all(|(key, val)| {
                    self.headers.iter().any(|header| {
                        header
                            .properties
                            .get(key)
                            .map(|header_val| header_val == val)
                            .unwrap_or(false)
                    })
                });

                self.skip_section = !(matches_tags && matches_props)
            }

            if !self.skip_section {
                let curr_header = self.headers.last().unwrap();
                if !self.args.todo.is_empty() {
                    let has_todo = self
                        .args
                        .todo
                        .iter()
                        .any(|x| curr_header.todo.as_ref().map_or(false, |y| y == x));
                    self.skip_section = self.skip_section || !has_todo;
                }

                if !self.args.priority.is_empty() {
                    let is_right_priority = self
                        .args
                        .priority
                        .iter()
                        .any(|x| curr_header.priority.as_ref().map_or(false, |y| x == y));

                    self.skip_section = self.skip_section || !is_right_priority;
                }

                if let Some(priority) = &self.args.priority_lt {
                    let is_lt_than = curr_header
                        .priority
                        .as_ref()
                        .map_or(false, |x| x < priority);
                    self.skip_section = self.skip_section || !is_lt_than;
                }

                if let Some(priority) = &self.args.priority_gt {
                    let is_gt_than = curr_header
                        .priority
                        .as_ref()
                        .map_or(false, |x| x > priority);
                    self.skip_section = self.skip_section || !is_gt_than;
                }

                if let Some(schedule) = &self.args.scheduled_at {
                    //println!("{:?}", curr_header.datetime);
                    self.skip_section = self.skip_section
                        || !curr_header.datetime.as_ref().map_or(false, |datetime| {
                            datetime.compare_with(schedule, PartialEq::eq, PartialEq::eq)
                        });
                }
            }
        }
    }

    fn parse_header(&mut self) -> Option<OrgHeader> {
        let header: Result<(OrgHeader, &str), _> =
            parsers::org_header_single().parse(self.current_line());
        if let Ok((org_header, _)) = header {
            // TODO: it might be good if user does not search for
            // these, simply don't parse them ex. if --prop does not
            // exist in args, simply skip parse_org_props() call etc.
            let datetime = self.parse_org_date_time();
            let properties = self.parse_org_props();
            Some(OrgHeader {
                properties,
                datetime,
                ..org_header
            })
        } else {
            None
        }
    }

    fn parse_org_date_time(&mut self) -> Option<OrgDateTime> {
        // Only ISO 8601 dates are supported
        // TODO: handle plain timestamps after headers
        let has_schedule = self
            .iter
            .peek()
            .map(|(_, x)| x.starts_with_i("DEADLINE:") || x.starts_with_i("SCHEDULED:"))
            .unwrap_or(false);

        if has_schedule {
            let (_, line_date) = self.iter.next().unwrap();
            let result: Result<(OrgDateTime, &str), _> =
                parsers::org_date_time().parse(line_date.as_str());
            result.ok().map(|x| x.0)
        } else {
            None
        }
    }

    fn parse_org_props(&mut self) -> HashMap<String, String> {
        let has_props = self
            .iter
            .peek()
            .map(|(_, x)| x.starts_with_i(":PROPERTIES:"))
            .unwrap_or(false);
        let mut props: HashMap<String, String> = HashMap::new();

        if has_props {
            self.iter.next(); // Consume :PROPERTIES:

            while let Some((_, prop)) = self.iter.next() {
                if prop.starts_with_i(":END:") {
                    return props;
                } else {
                    let result: Result<((String, String), &str), _> =
                        parsers::org_property().parse(&prop);
                    if let Ok(((key, val), _)) = result {
                        props.insert(key, val);
                    } else {
                        // Probably :PROPERTIES: block does not have
                        // :END: (I just assumed this to not to
                        // consume whole file, it might just be a bad
                        // property line) so just return what we just
                        // have found so far
                        return props;
                    }
                }
            }
        }

        props
    }

    fn current_line(&self) -> &str {
        self.iter_current.1.as_str()
    }
}
