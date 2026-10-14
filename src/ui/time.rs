use chrono::{DateTime, Datelike as _, Local, TimeZone as _};

fn local(ms: i64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms).single()
}

pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

pub fn list_stamp(ms: i64) -> String {
    let Some(time) = local(ms) else { return String::new() };
    let now = Local::now();
    let days = (now.date_naive() - time.date_naive()).num_days();
    match days {
        0 => time.format("%-I:%M %p").to_string(),
        1 => "Yesterday".to_string(),
        2..=6 => time.format("%A").to_string(),
        _ => time.format("%-m/%-d/%y").to_string(),
    }
}

pub fn divider(ms: i64) -> String {
    let Some(time) = local(ms) else { return String::new() };
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
    let Some(previous) = previous else { return true };
    let (Some(a), Some(b)) = (local(previous), local(current)) else { return false };
    a.date_naive() != b.date_naive() || current - previous >= 60 * 60 * 1000
}
