use combine::*;
use chrono::prelude::*;
use combine::parser::char::*;
use combine::Stream;
use combine::error::ParseError;

use crate::org::datetime::{OrgDatePlan, OrgDateTime};

/// Parse `HH:MM`.
pub fn hour<Input>() -> impl Parser<Input, Output = (u32, u32)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        token(':'),
        count_min_max(2, 2, digit()).map(|x: String| x.parse::<u32>().unwrap())
    ).map(|(h, _, m)| (h, m))
}

/// Parse `HH:MM-HH:MM`. Second part is optional.
pub fn hour_range<Input>() -> impl Parser<Input, Output = ((u32, u32), Option<(u32, u32)>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        hour(),
        optional(token('-').and(hour()))
    ).map(|(x, y)| (x, y.map(|(_, a)| a)))
}


pub fn org_date_time<Input>() -> impl Parser<Input, Output = OrgDateTime>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let invertal_parser = many1(satisfy(|x| x != '>' && x != ']'));

    (
        spaces().silent(),
        many1(letter()).map(|x: String| match x.as_str() {
            "DEADLINE" => OrgDatePlan::Deadline,
            "SCHEDULED" => OrgDatePlan::Scheduled,
            _ => OrgDatePlan::Plain, // FIXME: this is wrong, it should not happen
        }),
        token(':'),
        spaces().silent(),
        choice((token('<'), token('['))).map(|c| c == '<'), // < means active, [ means inactive
        count(4, digit()).map(|x: String| x.parse::<i32>().unwrap()),
        token('-'),
        count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        token('-'),
        count(2, digit()).map(|x: String| x.parse::<u32>().unwrap()),
        spaces(),
        count(3, letter()).map(|x: String| x),
        spaces().silent(),
        optional(hour_range()).map(|hour| hour.unwrap_or(((0, 0), None))),
        spaces().silent(),
        optional(invertal_parser),
        choice((token(']'), token('>'))),
    ).map(|(_, date_plan, _, _, is_active, year, _, month, _, day, _, _day_str, _, hour, _, invertal, _)| OrgDateTime {
        is_active,
        date_plan,
        date_start: Utc.ymd(year, month, day).and_hms(hour.0.0, hour.0.1, 0),
        date_end: hour.1.map(|end| Utc.ymd(year, month, day).and_hms(end.0, end.1, 0)),
        invertal,
    })
}

pub fn org_tags<Input>() -> impl Parser<Input, Output = Vec<String>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        spaces().silent(),
        token(':'),
        sep_end_by1(many1(alpha_num()), token(':')).map(|xs: Vec<String>| xs.iter().map(|x| x.chars().rev().collect()).collect()),
        spaces().silent(),
    ).map(|(_, _, tags, _)| tags)
}

pub fn org_property<Input>() -> impl Parser<Input, Output = (String, String)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let non_colon = satisfy(|x| x != ':');
    (
        between(char(':'), char(':'), many1(non_colon)),
        spaces().silent(),
        many(any())
    ).map(|(key, _, val)| (key, val))
}


#[test]
fn test_hour() {
    assert_eq!(hour().parse("13:27").unwrap().0, (13, 27));
    assert_eq!(hour().parse("15:42").unwrap().0, (15, 42));
}

#[test]
fn test_hour_range() {
    assert_eq!(hour_range().parse("13:27").unwrap().0, ((13, 27), None));
    assert_eq!(hour_range().parse("13:27-14:30").unwrap().0, ((13, 27), Some((14, 30))));
    assert_eq!(hour_range().parse("19:00-19:30").unwrap().0, ((19, 00), Some((19, 30))));
}

#[test]
fn test_org_date_time() {
    assert_eq!(org_date_time().parse("DEADLINE: <2020-12-24 Thu>").unwrap().0, OrgDateTime {
        is_active: true,
        date_plan: OrgDatePlan::Deadline,
        date_start: Utc.ymd(2020, 12, 24).and_hms(0, 0, 0),
        date_end: None,
        invertal: None,
    });

    assert_eq!(org_date_time().parse("DEADLINE: <2020-12-24 Thu 13:30>").unwrap().0, OrgDateTime {
        is_active: true,
        date_plan: OrgDatePlan::Deadline,
        date_start: Utc.ymd(2020, 12, 24).and_hms(13, 30, 0),
        date_end: None,
        invertal: None,
    });

    assert_eq!(org_date_time().parse("DEADLINE: [2020-12-24 Thu 13:30]").unwrap().0, OrgDateTime {
        is_active: false,
        date_plan: OrgDatePlan::Deadline,
        date_start: Utc.ymd(2020, 12, 24).and_hms(13, 30, 0),
        date_end: None,
        invertal: None,
    });

    assert_eq!(org_date_time().parse("DEADLINE: [2020-12-24 Thu 13:30 +1y]").unwrap().0, OrgDateTime {
        is_active: false,
        date_plan: OrgDatePlan::Deadline,
        date_start: Utc.ymd(2020, 12, 24).and_hms(13, 30, 0),
        date_end: None,
        invertal: Some("+1y".into()),
    });

    assert_eq!(org_date_time().parse("DEADLINE: [2020-12-24 Thu 13:30-22:35 +1y]").unwrap().0, OrgDateTime {
        is_active: false,
        date_plan: OrgDatePlan::Deadline,
        date_start: Utc.ymd(2020, 12, 24).and_hms(13, 30, 0),
        date_end: Some(Utc.ymd(2020, 12, 24).and_hms(22, 35, 0)),
        invertal: Some("+1y".into()),
    });
}

#[test]
fn test_org_tags() {
    assert_eq!(org_tags().parse("  :tset:2tset:3tset:         tser **").unwrap(),
               (vec!["test".into(), "test2".into(), "test3".into()], "tser **"))
}
