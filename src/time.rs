use chrono::prelude::*;
use nom::character::complete::digit1;

type Time<'a> = (u32, Option<u32>, &'a [u8]);

named!(parse_u32<u32>, flat_map!(digit1, parse_to!(u32)));
named!(parse_i32<i32>, flat_map!(digit1, parse_to!(i32)));

named!(
    time<Time>,
    tuple!(
        parse_u32,
        opt!(preceded!(tag!(":"), parse_u32)),
        alt!(tag!("pm") | tag!("am"))
    )
);

type Date = (u32, Option<u32>, Option<i32>);

named!(
    date<Date>,
    tuple!(
        terminated!(parse_u32, tag!("|")),
        opt!(terminated!(parse_u32, tag!("|"))),
        opt!(terminated!(parse_i32, tag!("|")))
    )
);

named!(date_time<(Option<Date>, Time)>, tuple!(opt!(date), time));

named!(
    duration_el<(u32, &[u8])>,
    pair!(parse_u32, alt!(tag!("h") | tag!("m") | tag!("d")))
);

named!(duration<Vec<(u32, &[u8])>>, many1!(duration_el));

#[derive(Debug)]
enum AlarmParsed<'a> {
    Duration(Vec<(u32, &'a [u8])>),
    DateTime((Option<Date>, Time<'a>)),
}

named!(
    parse_alarm<AlarmParsed>,
    alt!(
        map!(duration, |res: Vec<(u32, &[u8])>| AlarmParsed::Duration(
            res
        )) | map!(date_time, |res| AlarmParsed::DateTime(res))
    )
);

fn parse_parsed_time(parsed: AlarmParsed) -> NaiveDateTime {
    use chrono::Duration;
    match parsed {
        AlarmParsed::Duration(values) => {
            let mut at: DateTime<Local> = Local::now();
            values.iter().for_each(|(val, unit)| {
                let unit = String::from_utf8_lossy(unit).to_string();
                match unit.as_ref() {
                    "m" => {
                        at = at + Duration::minutes(*val as i64);
                    }
                    "h" => {
                        at = at + Duration::hours(*val as i64);
                    }
                    "d" => {
                        at = at + Duration::days(*val as i64);
                    }
                    _ => unreachable!(),
                }
            });
            at.naive_local()
        }
        AlarmParsed::DateTime((date, time)) => {
            let local: DateTime<Local> = Local::now();

            let date = if let Some((d, m, y)) = date {
                (d, m.unwrap_or(local.month()), y.unwrap_or(local.year()))
            } else {
                (local.day(), local.month(), local.year())
            };

            let (mut hr, min, mode) = time;

            if mode == b"pm" {
                hr += 12;
            }

            let min = min.unwrap_or(0);

            NaiveDate::from_ymd(date.2, date.1, date.0).and_hms(hr, min, 0)
        }
    }
}

pub fn parse_time(time: String) -> NaiveDateTime {
    parse_parsed_time(parse_alarm(time.as_bytes()).unwrap().1)
}

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    // fn test_parse_time() {
    //     // dbg!(parse_alarm(b"1:30pm"));
    //     // dbg!(parse_alarm(b"1pm"));
    //     // dbg!(time(b"3:1 "));
    //     // dbg!(parse_alarm(b"1|1 "));
    //     dbg!(parse_alarm(b"1|2|2000|3pm "));
    //     dbg!(parse_time(parse_alarm(b"1|2|2000|3pm ").unwrap().1));
    // }
}
