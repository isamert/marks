use fuzzy_matcher::skim::SkimMatcherV2;
use std::fs::File;
use std::io::{BufRead, BufReader};
use walkdir::DirEntry;
use walkdir::WalkDir;

use crate::{args::Args, searcher::{Searcher, new_searcher}};
use crate::result::SearchResult;
use crate::utils::file_utils;

pub struct Marks<'a> {
    pub args: &'a Args,
}

#[derive(Debug, Clone)]
pub enum DocType {
    Markdown,
    OrgMode,
}

impl<'a> Marks<'a> {
    pub fn new(args: &'a Args) -> Marks {
        Marks { args }
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

    pub fn search_file(&self, file: &DirEntry) -> Option<Vec<SearchResult>> {
        let mut searcher = new_searcher(self.args, file)?;
        Some(searcher.search())
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
}
