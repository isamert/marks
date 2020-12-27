use std::collections::HashMap;

use crate::args::Args;
use crate::org::datetime::OrgDateTime;

#[derive(Debug, Eq, PartialEq)]
pub struct OrgPriority(pub String);

impl PartialOrd for OrgPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.0.chars().all(|x| x.is_alphabetic()) { // A > B
            other.0.partial_cmp(&self.0)
        } else if self.0.chars().all(|x| x.is_digit(10)) { // 2 > 1
            self.0.parse::<u32>().unwrap_or(0).partial_cmp(&other.0.parse::<u32>().unwrap_or(0))
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OrgTodo {
    TODO,
    DONE,
    Other(String),
}

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
    /// TODO state
    pub todo: Option<OrgTodo>,
    /// The priority, like [#...], ... being anything
    pub priority: Option<OrgPriority>,
}

#[test]
fn test_priority_ordering() {
    assert!(OrgPriority("A".into()) > OrgPriority("B".into()));
    assert!(OrgPriority("3".into()) > OrgPriority("2".into()));
    assert!(OrgPriority("15".into()) > OrgPriority("13".into()));
    assert!(OrgPriority("A".into()) == OrgPriority("A".into()));
}
