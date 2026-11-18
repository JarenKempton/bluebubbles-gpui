use chrono::{DateTime, Datelike as _, Local, TimeZone as _};

fn local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

pub fn relative(ms: i64) -> String {
    relative_between(ms, now_ms())
}

fn relative_between(ms: i64, now: i64) -> String {
    let minutes = (now - ms).max(0) / 60_000;
    match minutes {
        0 => "now".to_string(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        1440..=10079 => format!("{}d", minutes / 1440),
        _ => format!("{}w", minutes / 10080),
    }
}

pub fn clock(ms: i64) -> String {
    local(ms)
        .map(|t| t.format("%-I:%M %p").to_string())
        .unwrap_or_default()
}

pub fn divider(ms: i64) -> String {
    let Some(time) = local(ms) else {
        return String::new();
    };
    let now = Local::now();
    let days = (now.date_naive() - time.date_naive()).num_days();
    let clock = time.format("%-I:%M %p");
    match days {
        0 => format!("Today {clock}"),
        1 => format!("Yesterday {clock}"),
        2..=6 => format!("{} {clock}", time.format("%A")),
        _ if time.year() == now.year() => format!("{} {clock}", time.format("%a, %b %-d")),
        _ => format!("{} {clock}", time.format("%b %-d, %Y")),
    }
}

pub fn needs_divider(previous: Option<i64>, current: i64) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    let (Some(a), Some(b)) = (local(previous), local(current)) else {
        return false;
    };
    a.date_naive() != b.date_naive() || current - previous >= 60 * 60 * 1000
}

#[cfg(test)]
mod tests {
    use super::relative_between;

    #[test]
    fn relative_uses_the_largest_whole_unit() {
        let minute = 60_000;
        let now = 100 * 7 * 24 * 60 * minute;
        assert_eq!(relative_between(now - 30_000, now), "now");
        assert_eq!(relative_between(now - 5 * minute, now), "5m");
        assert_eq!(relative_between(now - 3 * 60 * minute, now), "3h");
        assert_eq!(relative_between(now - 2 * 1440 * minute, now), "2d");
        assert_eq!(relative_between(now - 21 * 1440 * minute, now), "3w");
    }
}
