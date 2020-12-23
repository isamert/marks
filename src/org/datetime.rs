use chrono::prelude::*;

#[derive(Debug, Eq, PartialEq)]
pub enum OrgDatePlan {
    /// SCHEDULED dates
    Scheduled,
    /// DEADLINE dates
    Deadline,
    /// Just plain dates, no DEADLINE or SCHEDULED prefix
    Plain,
}

/// Some possible formats:
/// <2003-09-16 Tue>
/// <2003-09-16 Tue 12:00-12:30>
/// <2003-09-16 Tue 12:00>--<2003-09-19 Tue 14:30>
#[derive(Debug, Eq, PartialEq)]
pub struct OrgDateTime {
    /// <...> is for active dates, [...] is for passive dates.
    pub is_active: bool,
    /// Is it SCHEDULED, DEADLINE or just plain date?
    pub date_plan: OrgDatePlan,
    /// First date found in the org datetime.
    pub date_start: DateTime<Utc>,
    /// Second date found in the org datetime. Following formats has the second date:
    /// <...>--<...>
    /// <... HH:MM-HH-MM>.
    pub date_end: Option<DateTime<Utc>>,
    /// Invertal. Not quite useful at this point.
    /// https://orgmode.org/manual/Repeated-tasks.html
    pub invertal: Option<String>,
}
