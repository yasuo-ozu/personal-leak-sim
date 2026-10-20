//! 日付は 1970-01-01 からの日数（i32）で扱う。

pub fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}

pub fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

const WEEKDAYS: [&str; 7] = ["木", "金", "土", "日", "月", "火", "水"];

pub fn fmt(day: i32) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{}年{}月{}日", y, m, d)
}

pub fn fmt_wd(day: i32) -> String {
    format!("{}（{}）", fmt(day), WEEKDAYS[day.rem_euclid(7) as usize])
}

pub fn fmt_ym(day: i32) -> String {
    let (y, m, _) = civil_from_days(day);
    format!("{}年{}月", y, m)
}

pub fn year(day: i32) -> i32 {
    civil_from_days(day).0
}

pub fn md(day: i32) -> (u32, u32) {
    let (_, m, d) = civil_from_days(day);
    (m, d)
}

pub fn age_at(birth: i32, day: i32) -> i32 {
    let (by, bm, bd) = civil_from_days(birth);
    let (y, m, d) = civil_from_days(day);
    let mut a = y - by;
    if (m, d) < (bm, bd) {
        a -= 1;
    }
    a
}

/// `after` 以降で最初に来る誕生日
pub fn next_birthday(birth: i32, after: i32) -> i32 {
    let (_, bm, bd) = civil_from_days(birth);
    let y = year(after);
    for yy in [y, y + 1] {
        let d = if bm == 2 && bd == 29 { days_from_civil(yy, 3, 1) } else { days_from_civil(yy, bm, bd) };
        if d >= after {
            return d;
        }
    }
    days_from_civil(y + 1, bm, bd)
}

/// 学年の起点年（4/2〜翌4/1生まれが同学年）
pub fn cohort_year(birth: i32) -> i32 {
    let (y, m, d) = civil_from_days(birth);
    if (m, d) >= (4, 2) {
        y
    } else {
        y - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        for z in [-1000, 0, 365, 20000, 21000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(age_at(days_from_civil(2010, 4, 12), days_from_civil(2027, 4, 11)), 16);
        assert_eq!(age_at(days_from_civil(2010, 4, 12), days_from_civil(2027, 4, 12)), 17);
        assert_eq!(cohort_year(days_from_civil(2011, 4, 1)), 2010);
    }
}
