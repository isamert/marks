use std::{error::Error, path::PathBuf};
use structopt::StructOpt;

use crate::{org::header::{OrgPriority, OrgTodo}, query::Query};

#[derive(Debug, StructOpt)]
#[structopt(name = "marks")]
/// A search-engine like search tool for markdown and org-mode files.
pub struct Args {
    /// Activate debug mode
    #[structopt(short, long)]
    pub debug: bool,

    // TODO: newlines for the help string?
    /// The query.
    ///
    /// An example query may look like this:
    ///
    ///    '"this" is a `(test|trial)` query -badword'
    ///
    /// This query requires
    ///
    ///   - the word "this" to be either in the title hierarchy or in the line.
    ///
    ///   - regex "(test|trial)" to either in the title hierarchy or in the line.
    ///
    ///   - "badword" to be not in the title hierarchy or the line itself.
    ///
    /// Rest of the characters are matched in fuzzy fashion.
    #[structopt(short, long, parse(try_from_str = parse_query))]
    pub query: Query,

    /// Where to search for.
    #[structopt(short, long, env = "PWD", parse(try_from_str = parse_path))]
    pub path: PathBuf,

    /// How many results do you want?
    #[structopt(short, long)]
    pub count: Option<usize>,

    /// TODO states.
    #[structopt(long, parse(try_from_str = parse_todos))]
    pub todo: Vec<OrgTodo>,

    /// List of priorities. Note that items without priorites will not match if you use this option.
    #[structopt(long, parse(try_from_str = parse_priority))]
    pub priority: Vec<OrgPriority>,

    /// Maximum priority.
    #[structopt(long, parse(try_from_str = parse_priority))]
    pub priority_lt: Option<OrgPriority>,

    /// Minimum priority.
    #[structopt(long, parse(try_from_str = parse_priority))]
    pub priority_gt: Option<OrgPriority>,

    /// List of tags that headers should contain. Headers inherit parents tags.
    #[structopt(long)]
    pub tagged: Vec<String>,

    /// List of key=value pairs. If given, headers should contain given property in their property list.
    #[structopt(long, parse(try_from_str = parse_props))]
    pub prop: Vec<(String, String)>,

    /// Print only matching headers.
    /// This does not change anything in matching algorithm, only hides the content from the results.
    #[structopt(long)]
    pub only_headers: bool,

    /// List of extensions for org files.
    #[structopt(short, long, default_value = "org")]
    pub org_extension: Vec<String>,

    /// List of extensions for org files.
    #[structopt(short, long, default_value = "md")]
    pub md_extension: Vec<String>,

    /// Don't search for org files.
    #[structopt(long)]
    pub no_org: bool,

    /// Don't search for markdown files.
    #[structopt(long)]
    pub no_markdown: bool,

    /// Whether to search in files too.
    #[structopt(long)]
    pub search_filename: bool,

    /// Don't use colors for the output.
    #[structopt(long)]
    pub no_color: bool,

    /// Don't include headers to output.
    #[structopt(long)]
    pub no_headers: bool,

    /// A seperator to insert between headers while outputting.
    #[structopt(long, default_value = "/")]
    pub header_seperator: String,

    /// List folder names to blacklist
    #[structopt(long)]
    pub blacklist_folder: Vec<String>,
}

fn parse_props<'a>(s: &'a str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid PROP=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

fn parse_query<'a>(s: &'a str) -> Result<Query, impl Error + 'a> {
    Query::new(s)
}

fn parse_path<'a>(s: &'a str) -> Result<PathBuf, impl Error + 'a> {
    PathBuf::from(s).canonicalize()
}

fn parse_todos<'a>(s: &'a str) -> Result<OrgTodo, String> {
    Ok(match s.to_uppercase().as_ref() {
        "TODO" => OrgTodo::TODO,
        "DONE" => OrgTodo::DONE,
        x => OrgTodo::Other(x.into())
    })
}

fn parse_priority<'a>(s: &'a str) -> Result<OrgPriority, String> {
    Ok(OrgPriority(s.into()))
}
