use std::io;
use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use walkdir::DirEntry;
use regex::Regex;
use std::collections::HashMap;
use rayon::prelude::*;

use crate::args::Args;
use crate::query::Query;
use crate::utils::file_utils;
use crate::parser::GentleIterator;
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

enum DocType {
    Markdown,
    OrgMode,
}

impl<'a> Marks<'a> {
    pub fn new(args: &'a Args) -> Marks {
        // TODO: parametrize this
        let matcher = SkimMatcherV2::default();

        Marks {
            query: Query::new(&args.query),
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

        for (index, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            let header_info = self.parse_header(&doc_type, &line);
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
                    headers[depth - 1] = header;
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

    fn parse_header(&self, typ: &DocType, line: &str) -> Option<(usize, String)> {
        let x = match typ {
            DocType::Markdown => '#',
            DocType::OrgMode => '*',
        };

        let mut chars = line.chars();
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

        return Some((depth, chars.collect()));
    }
}
