use std::{collections::HashMap, iter::Peekable};

use chrono::prelude::*;
use combine::{error::ParseError, parser::{range::take_while, repeat::take_until}};
use combine::parser::char::*;
use combine::Stream;
use combine::*;
use indoc::indoc;

use crate::{marks::DocType, org::datetime::{OrgDatePlan, OrgDateTime}, result::SearchResult};
use crate::org::header::*;


pub fn blanks<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>, {
    many(char(' ')).map(|_: String| ())
}

/// Parse `HH:MM`.
pub fn hour<Input>() -> impl Parser<Input, Output = (u32, u32)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        token(':'),
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
    )
        .map(|(h, _, m)| (h, m))
}

/// Parse `HH:MM-HH:MM`. Second part is optional.
pub fn hour_range<Input>() -> impl Parser<Input, Output = ((u32, u32), Option<(u32, u32)>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (hour(), optional(token('-').and(hour()))).map(|(x, y)| (x, y.map(|(_, a)| a)))
}


/// Parse `HH:MM-HH:MM`. Second part is optional.
pub fn date_time_range<Input>() -> impl Parser<Input, Output = (DateTime<Utc>, Option<DateTime<Utc>>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        count(4, digit()).map(|x: String| x.parse::<i32>().unwrap()),
        token('-'),
        count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        token('-'),
        count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        blanks(),
        optional(count(3, letter()).map(|x: String| x)),
        blanks().silent(),
        optional(hour_range()).map(|hour| hour.unwrap_or(((0, 0), None))),
    ).map(|(year, _, month, _, day, _, _, _, hour)| (
        Utc.ymd(year, month, day).and_hms(hour.0 .0, hour.0 .1, 0),
        hour.1.map(|end| Utc.ymd(year, month, day).and_hms(end.0, end.1, 0))
    ))
}

pub fn org_date_time<Input>() -> impl Parser<Input, Output = OrgDateTime>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let invertal_parser = many1(satisfy(|x| x != '>' && x != ']'));

    (
        choice((string("DEADLINE:"), string("SCHEDULED:"), string(""))).map(|x| match x {
            "DEADLINE:" => OrgDatePlan::Deadline,
            "SCHEDULED:" => OrgDatePlan::Scheduled,
            _ => OrgDatePlan::Plain,
        }),
        blanks().silent(),
        choice((token('<'), token('['))).map(|c| c == '<'), // < means active, [ means inactive
        date_time_range(),
        blanks().silent(),
        optional(invertal_parser),
        choice((token(']'), token('>'))),
    )
        .map(|(date_plan, _, is_active, (date_start, date_end), _, invertal, _,)| OrgDateTime {
            is_active,
            date_plan,
            date_start,
            date_end,
            invertal,
        })
}

pub fn org_tags<Input>() -> impl Parser<Input, Output = Vec<String>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        token(':'),
        sep_end_by1(many1(alpha_num()), token(':')),
    )
        .map(|(_, tags)| tags)
}

pub fn org_todo<Input>() -> impl Parser<Input, Output = (Option<OrgTodo>, Option<OrgPriority>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let org_priority = (token('['), token('#'), many(alpha_num()), token(']'))
        .map(|(_, _, priority, _)| OrgPriority(priority));

    (
        optional(attempt(many1(upper()).and(char(' ')).map(
            |(x, _): (String, _)| match x.as_str() {
                "TODO" => OrgTodo::TODO,
                "DONE" => OrgTodo::DONE,
                // TODO: check if `x` is a legit TODO keyword or not, otherwise bail
                _ => OrgTodo::Other(x),
            },
        ))),
        blanks().silent(),
        optional(attempt(org_priority)),
        blanks().silent(),
    )
        .map(|(todo, _, priority, _)| (todo, priority))
}

pub fn org_title<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // FIXME This is not right, it can't parse titles with colon in them
    many1(none_of("\n:".chars())).map(|it: String| it.trim().into())
}

pub fn org_property<Input>() -> impl Parser<Input, Output = (String, String)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let non_colon = none_of("\n:".chars());
    (
        between(char(':'), char(':'), many1(non_colon)),
        blanks(),
        many1(satisfy(|c| c != '\n')).map(|x: String| x.trim().into()),
    )
        .map(|(key, _, val)| (key, val))
}

pub fn org_properties<Input>() -> impl Parser<Input, Output = HashMap<String, String>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string(":PROPERTIES:\n"),
        sep_end_by(attempt(org_property()), newline()),
        string(":END:")
    )
        .map(|(_, props, _)| props)
}

pub fn org_header_prefix<Input>() -> impl Parser<Input, Output = usize>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>, {
    many1(char('*')).and(char(' ')).map(|(it, _): (String, _)| it.len())
}

pub fn org_header<Input>() -> impl Parser<Input, Output = OrgHeader>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>, {

    (
        org_header_prefix(),
        blanks(),
        org_todo(),
        org_title(),
        blanks(),
        optional(org_tags()).map(|x| x.unwrap_or(vec![])),
        optional(attempt(newline().and(org_date_time()).map(|it| it.1))),
        optional(attempt(newline().and(org_properties())).map(|it| it.1)).map(|it| it.unwrap_or(HashMap::new()))
    ).map(|(depth, _, (todo, priority), content, _, tags, datetime, properties)|
          OrgHeader {
              depth,
              content,
              properties,
              tags,
              datetime,
              todo,
              priority,
          })
}

pub fn org_section<Input>() -> impl Parser<Input, Output = Vec<(OrgHeader, Vec<String>)>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>, {

    let new_header = attempt(newline().and(attempt(org_header_prefix())).map(|_| ()));

    many1(
        (
            org_header(),
            take_until(new_header.or(eof())).map(|it: String| it.split("\n").map(|it| it.to_string()).collect()),
            newline().map(|_| ()).or(eof()),
        ).map(|(x, y, _)| (x,y))
    )
}


#[test]
fn test_org_tags() {
    assert_eq!(
        org_tags()
            .parse(":test:tag:tag3:")
            .unwrap(),
        (
            vec!["test".into(), "tag".into(), "tag3".into()],
            ""
        )
    )
}

#[test]
fn test_org_property() {
    assert_eq!(
        org_property()
            .parse(":TEST: value")
            .unwrap(),
        (
            ("TEST".into(), "value".into()),
            ""
        )
    );

    assert_eq!(
        org_property()
            .parse(":another:   value  ")
            .unwrap(),
        (
            ("another".into(), "value".into()),
            ""
        )
    );
}

#[test]
fn test_org_properties() {
    assert_eq!(
        org_properties()
            .easy_parse(":PROPERTIES:\n:TEST: value\n:TEST2: another value\n:END:")
            .unwrap(),
        (
            [("TEST".into(), "value".into()),
             ("TEST2".into(), "another value".into())].iter().cloned().collect(),
            ""
        )
    );

    assert_eq!(
        org_properties()
            .easy_parse(":PROPERTIES:\n:RATING: 10/10\n:END:")
            .unwrap(),
        (
            [("RATING".into(), "10/10".into())].iter().cloned().collect(),
            ""
        )
    );

    assert_eq!(
        org_properties()
            .easy_parse(":PROPERTIES:\n:END:")
            .unwrap(),
        (
            HashMap::new(),
            ""
        )
    );
}

#[test]
fn test_org_todo() {
    assert_eq!(
        org_todo().parse("TODO The Ego and Its Own").unwrap(),
        ((Some(OrgTodo::TODO), None), "The Ego and Its Own")
    );

    assert_eq!(
        org_todo().parse("DONE [#B] The German Ideology").unwrap(),
        (
            (Some(OrgTodo::DONE), Some(OrgPriority("B".into()))),
            "The German Ideology"
        )
    );

    assert_eq!(
        org_todo().parse("PROG [#33] hehe").unwrap(),
        (
            (Some(OrgTodo::Other("PROG".into())), Some(OrgPriority("33".into()))),
            "hehe"
        )
    );
}

#[test]
fn test_org_header() {
    assert_eq!(
        org_header().easy_parse("** TODO [#B] The Ego and Its Own").unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec![],
            properties: HashMap::new(),
            datetime: None,
            todo: Some(OrgTodo::TODO),
            priority: Some(OrgPriority("B".into())),
        }
    );

    let with_deadline = indoc! {"
        ** TODO [#B] The Ego and Its Own :test:tags:
        DEADLINE: <2021-08-28 Sat>
    "};

    assert_eq!(
        org_header().easy_parse(with_deadline).unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec!["test".into(), "tags".into()],
            properties: HashMap::new(),
            datetime: Some(OrgDateTime {
                date_start: Utc.ymd(2021, 8, 28).and_hms(0, 0, 0),
                date_plan: OrgDatePlan::Deadline,
                ..Default::default()
            }),
            todo: Some(OrgTodo::TODO),
            priority: Some(OrgPriority("B".into())),
        }
    );

    let with_deadline_and_props = indoc! {"
        ** TODO The Ego and Its Own :test:tags:
        DEADLINE: <2021-08-28 Sat>
        :PROPERTIES:
        :RATING: 10/10
        :END:
    "};

    assert_eq!(
        org_header().easy_parse(with_deadline_and_props).unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec!["test".into(), "tags".into()],
            properties: [("RATING".into(), "10/10".into())].iter().cloned().collect(),
            datetime: Some(OrgDateTime {
                date_start: Utc.ymd(2021, 8, 28).and_hms(0, 0, 0),
                date_plan: OrgDatePlan::Deadline,
                ..Default::default()
            }),
            todo: Some(OrgTodo::TODO),
            priority: None,
        }
    );

    let with_props_and_tags = indoc! {"
        ** [#B] The Ego and Its Own :test:tags:
        :PROPERTIES:
        :RATING: 10/10
        :END:
    "};

    assert_eq!(
        org_header().easy_parse(with_props_and_tags).unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec!["test".into(), "tags".into()],
            properties: [("RATING".into(), "10/10".into())].iter().cloned().collect(),
            datetime: None,
            todo: None,
            priority: Some(OrgPriority("B".into())),
        }
    );

    let with_props = indoc! {"
        ** The Ego and Its Own
        :PROPERTIES:
        :RATING: 10/10
        :END:
    "};

    assert_eq!(
        org_header().easy_parse(with_props).unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec![],
            properties: [("RATING".into(), "10/10".into())].iter().cloned().collect(),
            datetime: None,
            todo: None,
            priority: None,
        }
    );

    let with_content = indoc! {"
        ** TODO [#B] The Ego and Its Own :test:tags:
        some content
    "};

    assert_eq!(
        org_header().easy_parse(with_content).unwrap().0,
        OrgHeader {
            depth: 2,
            content: "The Ego and Its Own".into(),
            tags: vec!["test".into(), "tags".into()],
            properties: HashMap::new(),
            datetime: None,
            todo: Some(OrgTodo::TODO),
            priority: Some(OrgPriority("B".into())),
        }
    );
}

#[test]
fn test_org_section() {
    assert_eq!(
        org_section().easy_parse(indoc! {"
          * TODO [#B] Hey
          :PROPERTIES:
          :RATING: I'm getting bored
          :END:
          test content
          more content
          ** Hello
          with some content

          * Saaa :test1:test2:
          :PROPERTIES:
          :RATING: I r8 8/8
          :END:
          Hell
        "}).unwrap(),
        (
            vec![
                (
                    OrgHeader {
                        depth: 1,
                        content: "Hey".into(),
                        tags: vec![],
                        properties: [("RATING".into(), "I'm getting bored".into())].iter().cloned().collect(),
                        datetime: None,
                        todo: Some(OrgTodo::TODO),
                        priority: Some(OrgPriority("B".into())),
                    },
                    vec!["test content".into(), "more content".into()]
                ),
                (
                    OrgHeader {
                        depth: 2,
                        content: "Hello".into(),
                        tags: vec![],
                        properties: HashMap::new(),
                        datetime: None,
                        todo: None,
                        priority: None,
                    },
                    vec!["with some content".into()]
                ),
                (
                    OrgHeader {
                        depth: 1,
                        content: "SA".into(),
                        tags: vec!["test1".into(), "test2".into()],
                        properties: [("RATING".into(), "I r8 8/8".into())].iter().cloned().collect(),
                        datetime: None,
                        todo: None,
                        priority: None,
                    },
                    vec!["Hell".into()]
                )
            ],
            ""
        )
    );
}
