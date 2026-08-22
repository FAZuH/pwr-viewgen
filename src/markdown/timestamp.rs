use chrono::DateTime;
use chrono::Utc;

pub const TIMESTAMP_STYLES: &str = "tTdDfFR";

pub fn is_valid_style(style: char) -> bool {
    TIMESTAMP_STYLES.contains(style)
}

pub fn fmt(unix: i64, style: char) -> String {
    fmt_with(unix, style, Utc::now().timestamp())
}

pub fn fmt_with(unix: i64, style: char, now_unix: i64) -> String {
    let Some(target) = DateTime::from_timestamp(unix, 0) else {
        return String::new();
    };
    match style {
        't' => target.format("%-I:%M %p").to_string(),
        'T' => target.format("%-I:%M:%S %p").to_string(),
        'd' => target.format("%m/%d/%Y").to_string(),
        'D' => target.format("%B %-d, %Y").to_string(),
        'F' => target.format("%A, %B %-d, %Y %-I:%M %p").to_string(),
        'R' => relative(unix, now_unix),
        _ => target.format("%B %-d, %Y %-I:%M %p").to_string(),
    }
}

fn relative(unix: i64, now_unix: i64) -> String {
    let diff_secs = now_unix - unix;
    if diff_secs == 0 {
        return "now".to_owned();
    }
    let future = diff_secs < 0;
    let magnitude = diff_secs.unsigned_abs();
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;

    let (amount, unit): (u64, &str) = if magnitude < MINUTE {
        (magnitude, "second")
    } else if magnitude < HOUR {
        (magnitude / MINUTE, "minute")
    } else if magnitude < DAY {
        (magnitude / HOUR, "hour")
    } else if magnitude < MONTH {
        (magnitude / DAY, "day")
    } else if magnitude < YEAR {
        (magnitude / MONTH, "month")
    } else {
        (magnitude / YEAR, "year")
    };

    let noun = if amount == 1 {
        unit.to_owned()
    } else {
        format!("{unit}s")
    };
    if future {
        format!("in {amount} {noun}")
    } else {
        format!("{amount} {noun} ago")
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const SAT_JAN_1_2000: i64 = 946_684_800;
    const LEAP_DAY_2024: i64 = 1_709_164_800;

    #[test]
    fn fixed_styles_format_en_us_in_utc() {
        let cases: &[(char, &str)] = &[
            ('t', "12:00 AM"),
            ('T', "12:00:00 AM"),
            ('d', "01/01/1970"),
            ('D', "January 1, 1970"),
            ('f', "January 1, 1970 12:00 AM"),
            ('F', "Thursday, January 1, 1970 12:00 AM"),
        ];
        for (style, expected) in cases {
            assert_eq!(&fmt_with(0, *style, 0), expected, "style {style}");
        }
    }

    #[test]
    fn weekend_and_leap_day_anchor_formats() {
        assert_eq!(
            fmt_with(SAT_JAN_1_2000, 'F', 0),
            "Saturday, January 1, 2000 12:00 AM"
        );
        assert_eq!(fmt_with(SAT_JAN_1_2000, 'D', 0), "January 1, 2000");
        assert_eq!(fmt_with(LEAP_DAY_2024, 'd', 0), "02/29/2024");
        assert_eq!(fmt_with(LEAP_DAY_2024, 'D', 0), "February 29, 2024");
    }

    #[test]
    fn unknown_style_falls_back_to_default_f() {
        assert_eq!(
            fmt_with(0, 'x', 0),
            fmt_with(0, 'f', 0),
            "unknown styles must render like f"
        );
    }

    #[test]
    fn unrepresentable_instant_renders_empty() {
        assert_eq!(fmt_with(i64::MAX, 't', 0), "");
    }

    #[test]
    fn relative_style_buckets_past_and_future() {
        let day: i64 = 86_400;
        let cases: &[(i64, i64, &str)] = &[
            (100, 100, "now"),
            (100, 130, "30 seconds ago"),
            (100, 190, "1 minute ago"),
            (100, 100 + 2 * 3600, "2 hours ago"),
            (100, 100 + 3 * day, "3 days ago"),
            (100, 100 + 45 * day, "1 month ago"),
            (100, 100 + 400 * day, "1 year ago"),
            (100, 70, "in 30 seconds"),
            (100, 40, "in 1 minute"),
            (100, 100 - 7200, "in 2 hours"),
        ];
        for (unix, now, expected) in cases {
            assert_eq!(
                &fmt_with(*unix, 'R', *now),
                expected,
                "unix {unix} now {now}"
            );
        }
    }

    #[test]
    fn style_set_matches_discord_documented_set() {
        for style in TIMESTAMP_STYLES.chars() {
            assert!(is_valid_style(style));
        }
        assert!(!is_valid_style('x'));
        assert!(!is_valid_style('r'), "lowercase r is not a style");
    }
}
