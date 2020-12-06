mod parser;
mod query;
mod utils;

use std::io;
use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use clap;
use clap::{Arg};
use sublime_fuzzy::best_match;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use walkdir::DirEntry;
use regex::Regex;
use std::collections::HashMap;
use rayon::prelude::*;

#[macro_use]
extern crate lazy_static;

use self::query::Query;
use self::utils::FileUtils;

lazy_static! {
    static ref HEADER_REGEX_MD:  Regex = Regex::new(r"^#+ ").unwrap();
    static ref HEADER_REGEX_ORG: Regex = Regex::new(r"^\*+ ").unwrap();
}

#[derive(Debug)]
struct SearchResult {
    score: i64,
    line: usize,
    file_path: String,
    headers: Vec<String>,
    content: String,
}


#[derive(Debug)]
struct App<'a> {
    // Print out debug info?
    debug: bool,
    // Splitted version of the full_query
    query: Query,
    // How many results to show? None means show all.
    count: Option<usize>,
    // Search for markdown files?
    md: bool,
    // Search for org files?
    org: bool,
    // Where to search?
    path: &'a str,
    // org file extensions
    org_ext: Vec<&'a str>,
    // markdown file extensions
    md_ext: Vec<&'a str>,
    // Whether to search in filename or not
    search_filename: bool,
}

impl<'a> App<'a> {
    fn is_file_blacklisted(self: &'a App<'a>, entry: &DirEntry) -> bool {
        let blacklist = vec!["node_modules"];
        entry.file_name()
            .to_str()
            .map(|s| blacklist.contains(&s))
            .unwrap_or(false)
    }

    fn is_org_file(self: &'a App<'a>, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.org_ext.contains(&x),
            None => false
        }
    }

    fn is_md_file(self: &'a App<'a>, e: &DirEntry) -> bool {
        match e.path().extension().map(|x| x.to_str()).flatten() {
            Some(x) => self.md_ext.contains(&x),
            None => false
        }
    }

    fn find_files(self: &'a App<'a>) -> impl Iterator<Item = DirEntry> + 'a {
        WalkDir::new(self.path)
            .into_iter()
            .filter_entry(move |e| !FileUtils::is_hidden(e) && !self.is_file_blacklisted(e))
            .filter_map(|e| e.ok())
            .filter_map(move |e| {
                if e.file_type().is_file() {
                    if (self.org && self.is_org_file(&e)) || (self.md && self.is_md_file(&e)) {
                        return Some(e)
                    }
                }

                return None
            })
    }

    fn header_regex(&'a self, file: &DirEntry) -> &Regex {
        if self.is_md_file(file) {
            &HEADER_REGEX_MD
        } else {
            &HEADER_REGEX_ORG
        }
    }

    fn search_file(&'a self, file: &DirEntry) -> Option<Vec<SearchResult>> {
        let matcher = SkimMatcherV2::default();
        let filename = file.file_name().to_str()?;
        let header_regex = self.header_regex(file);

        let mut results = vec![];
        let reader = BufReader::new(File::open(file.path()).ok()?);
        let mut headers: Vec<String> = vec![];
        let mut last_depth = 0;
        for (index, line) in reader.lines().enumerate() {
            let line = line.unwrap();
            let is_header = header_regex.is_match(&line);

            if is_header {
                let info: Vec<&str> = line.splitn(2, ' ').collect();
                if info.len() != 2 {
                    continue
                }

                let depth = info[0].len();
                let header = info[1];

                //println!("depth: {}, header: {}, headers: {:?}", depth, header, headers);

                if depth > last_depth {
                    headers.push(header.to_string());
                } else if last_depth == depth {
                    let lastn = headers.len() - 1;
                    headers[lastn] = header.to_string();
                } else {
                    headers.resize(depth, String::new());
                    headers[depth - 1] = header.to_string();
                }
                last_depth = depth;
            }

            // Build the string to search on
            let mut full: String = headers.join("/");
            if !is_header {
                full.push_str(&line);
            }
            if self.search_filename {
                full.push_str(filename);
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
            let points = self.query.rest.iter().filter_map(|q| matcher.fuzzy_match(&full, &q)).collect::<Vec<_>>();
            if points.len() > 0 {
                results.push(SearchResult {
                    line: index + 1,
                    file_path: file.path().to_str()?.to_string(),
                    score: points.iter().sum::<i64>(),
                    headers: headers.to_vec(),
                    content: line,
                });
            }
        }

        return Some(results);
    }
}

fn main() -> Result<(), io::Error> {
    let matches = clap::App::new("marks")
        .version("0.1.0")
        .author("Isa Mert Gurbuz <isamert@protonmail.com>")
        .about("Org/markdown semantic file search.")
        .arg(Arg::with_name("path")
             .short("p")
             .long("path")
             .takes_value(true)
             .default_value("./")
             .help("Path to look org files for."))
        .arg(Arg::with_name("query")
             .short("q")
             .long("query")
             .takes_value(true)
             .required(true)
             .help("The query."))
        .arg(Arg::with_name("count")
             .short("c")
             .long("count")
             .takes_value(true)
             .default_value("Inf"))
        .arg(Arg::with_name("no-md")
             .long("no-md")
             .takes_value(false))
        .arg(Arg::with_name("no-org")
             .long("no-org")
             .takes_value(false))
        .arg(Arg::with_name("include-filename")
             .long("include-filename")
             .takes_value(false))
        .arg(Arg::with_name("debug")
             .short("d")
             .long("debug")
             .takes_value(false))
        .get_matches();


    let app = App {
        debug: matches.is_present("debug"),
        query: Query::new(matches.value_of("query").unwrap()),
        path: matches.value_of("path").unwrap_or("./"),
        org_ext: matches.values_of("org-extension").map(|x| x.collect()).unwrap_or(vec!["org"]),
        md_ext: matches.values_of("extension").map(|x| x.collect()).unwrap_or(vec!["md", "markdown"]),
        count: matches.value_of("count").map(|x| x.parse::<usize>().ok()).flatten(),
        org: !matches.is_present("no-org"),
        md: !matches.is_present("no-md"),
        search_filename: matches.is_present("search-filename")
    };

    println!("{:#?}", app);

    let mut results = app.find_files().collect::<Vec<_>>().par_iter().filter_map(|f| app.search_file(&f)).flatten().collect::<Vec<_>>();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.iter().take(5).for_each(|x| println!("{:#?}", x));
    println!("{}", app.find_files().collect::<Vec<_>>().len());

    Ok(())
}
