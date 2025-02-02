use chrono::prelude::*;

#[derive(Debug, Eq, PartialEq, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone)]
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

impl Default for OrgDateTime {
    fn default() -> Self {
        OrgDateTime {
            is_active: true,
            date_plan: OrgDatePlan::Plain,
            date_start: Utc::now(),
            date_end: None,
            invertal: None,
        }
    }
}

impl OrgDateTime {
    pub fn compare_with(&self, other: &Self, compare1: fn(&DateTime<Utc>, &DateTime<Utc>) -> bool, compare2: fn(&Date<Utc>, &Date<Utc>) -> bool) -> bool {
        let compare_only_dates = (other.date_start.hour(), other.date_start.minute(), other.date_start.second()) == (0,0,0);
        let is_same_plan = self.date_plan == other.date_plan;

        is_same_plan && if compare_only_dates {
            compare2(&self.date_start.date(), &other.date_start.date())
        } else {
            compare1(&self.date_start, &other.date_start)
        }
    }
}
