use std::collections::HashMap;

use crate::args::Args;
use crate::org::datetime::OrgDateTime;

#[derive(Debug)]
pub struct OrgHeader<'a> {
    /// Args
    pub args: &'a Args,
    /// On which line is the header found.
    pub line: usize,
    /// This usually means the count of # (for md) or * (for org) at the beginning of the header line.
    pub depth: usize,
    /// The header itself, stripped from tags or other annotations.
    pub content: String,
    /// Tags found in the header. Means nothing for markdown headers.
    pub tags: Vec<String>,
    /// Properties found in :PROPERTIES: block of an org header. Means nothing for markdown headers.
    pub properties: HashMap<String, String>,
    /// SCHEDULED/DEADLINE status of the header.
    pub datetime: Option<OrgDateTime>,
}
