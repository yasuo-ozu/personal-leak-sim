//! 人物データから「情報（Fact）」の値を作る。

use crate::data::*;
use crate::date;
use crate::model::*;
use crate::seed::{hash01, hash_str};
use crate::world::World;

/// 学年の表示
pub fn grade_label(p: &Person, now: i32) -> String {
    let (y, m, _) = date::civil_from_days(now);
    let school_year = if m >= 4 { y } else { y - 1 };
    let g = school_year - date::cohort_year(p.birth) - 6;
    match g {
        1..=6 => format!("小学{}年", g),
        7..=9 => format!("中学{}年", g - 6),
        10..=12 => format!("高校{}年", g - 9),
        13..=16 => format!("大学{}年相当", g - 12),
        _ => format!("{}歳", p.age_at(now)),
    }
}

fn with_geo(mut f: FactInit, lat: f64, lon: f64, r: f64) -> FactInit {
    f.geo = Some(Geo { lat, lon, r });
    f
}

/// 丁目の中心（家の位置は ±200m ほどずれる）
pub fn chome_center(w: &World, a: &Address) -> (f64, f64) {
    let t = &w.towns[a.town];
    let ang = a.chome as f64 * 1.3;
    (t.lat + ang.sin() * 0.0025, t.lon + ang.cos() * 0.003)
}

/// variant の意味はフィールドごとに異なる:
/// - Age: 0=年齢, 1=学年
/// - Town: 0=町名, 1=丁目まで
/// - HomePhone: 0=現住所, 1=実家
/// - Email: 0=個人, 1=勤務先, 2=学校, 3+n=個人のn番目
/// - Handle: プラットフォーム番号
/// - School: 0=現在/最終, 1=小学校, 2=中学校, 3=高校
/// - Workplace: 0=社名, 1=部署まで
/// - Card: 0=下4桁, 1=全桁
/// - Password: 0=判明, 1=偽ページで入力させた, 2=再設定して乗っ取り
/// - History: カテゴリ番号
pub fn value(w: &World, id: u32, field: Field, variant: u8, now: i32) -> Option<FactInit> {
    let p = w.p(id);
    let a = w.addr(id);
    let mk = |v: String| FactInit::new(field, v, id);
    let mut f = match field {
        Field::Name => mk(p.full_name()),
        Field::Kana => mk(p.kana()),
        Field::Alias => mk(p.stage.clone().unwrap_or_else(|| p.alias.clone())),
        Field::Gender => mk(p.gender.label().to_string()),
        Field::BirthDate => {
            let mut f = mk(date::fmt(p.birth));
            f.aux = p.birth as i64;
            f
        }
        Field::BirthMd => {
            let (m, d) = date::md(p.birth);
            let mut f = mk(format!("{}月{}日", m, d));
            f.aux = (m * 100 + d) as i64;
            f
        }
        Field::Age => {
            if variant == 1 {
                let y = date::cohort_year(p.birth);
                let mut f = mk(format!("{}（{}年4月〜{}年3月生まれの学年）", grade_label(p, now), y, y + 1));
                f.aux = y as i64;
                f.aux2 = -1;
                f
            } else {
                let age = p.age_at(now);
                let mut f = mk(format!("{}歳（{}時点）", age, date::fmt_ym(now)));
                f.aux = age as i64;
                f.aux2 = now as i64;
                f
            }
        }
        Field::Pref => {
            let pr = PREFS[a.pref];
            with_geo(mk(pr.0.to_string()), pr.1, pr.2, 40000.0)
        }
        Field::City => {
            let c = CITIES[a.city];
            with_geo(mk(format!("{}{}", PREFS[a.pref].0, c.1)), c.2, c.3, c.5 * 1000.0)
        }
        Field::Station => {
            let s = w.station_of(id);
            let mut f = with_geo(mk(format!("{}（{}）", s.name, CITIES[s.city].1)), s.lat, s.lon, 1500.0);
            f.key = s.name.clone();
            f
        }
        Field::Postal => {
            let t = &w.towns[a.town];
            let mut f = with_geo(mk(format!("〒{}（{}{}）", t.postal, CITIES[a.city].1, t.name)), t.lat, t.lon, 700.0);
            f.key = t.postal.clone();
            f
        }
        Field::Town => {
            let t = &w.towns[a.town];
            if variant == 1 {
                let (la, lo) = chome_center(w, a);
                let mut f = with_geo(mk(format!("{}{}{}丁目", CITIES[a.city].1, t.name, a.chome)), la, lo, 350.0);
                f.aux = a.chome as i64;
                f
            } else {
                with_geo(mk(format!("{}{}", CITIES[a.city].1, t.name)), t.lat, t.lon, 900.0)
            }
        }
        Field::Address => with_geo(mk(w.address_text(a)), a.lat, a.lon, 15.0),
        Field::OldAddress => {
            let ha = &w.households[p.hometown?].addr;
            let mut f = with_geo(mk(format!("{}（実家）", w.address_text(ha))), ha.lat, ha.lon, 15.0);
            f.key = w.address_text(ha);
            f.pref = Some(ha.pref);
            f
        }
        Field::Routine => mk(p.routine.clone()),
        Field::Mobile => mk(p.mobile.clone()?),
        Field::HomePhone => {
            let h = if variant == 1 { p.hometown? } else { p.household };
            let ph = w.households[h].phone.clone()?;
            let mut f = mk(if variant == 1 { format!("{}（実家）", ph) } else { ph.clone() });
            f.key = ph;
            f
        }
        Field::Email => mk(match variant {
            1 => p.work_email.clone()?,
            2 => p.school_email.clone()?,
            v if v >= 3 => p.emails.get((v - 3) as usize)?.clone(),
            _ => p.emails.first()?.clone(),
        }),
        Field::Handle => {
            let acc = p.account(variant)?;
            let mut f = mk(format!("{} @{}", PLATFORMS[variant as usize].0, acc.handle));
            f.key = acc.handle.clone();
            f.aux = variant as i64;
            f
        }
        Field::School => {
            let s = match variant {
                1 => p.past_schools.first()?.clone(),
                2 => p.past_schools.get(1)?.clone(),
                3 => p.past_schools.get(2)?.clone(),
                _ => p.school.clone()?,
            };
            let mut f = mk(if variant == 0 { s.clone() } else { format!("出身校: {}", s) });
            f.key = s;
            f
        }
        Field::Workplace => {
            let wp = p.workplace.clone()?;
            let mut f = mk(if variant == 1 { format!("{} {}", wp, p.dept.unwrap_or("（部署不明）")) } else { wp.clone() });
            f.key = wp;
            f
        }
        Field::Family => mk(w.family_text(id, now)?),
        Field::Hobby => mk(p.hobbies.iter().map(|&h| HOBBIES[h]).collect::<Vec<_>>().join("、")),
        Field::Idol => {
            let (g, m) = p.idol?;
            mk(format!("{}（推し: {}）", IDOL_GROUPS[g].0, IDOL_GROUPS[g].1[m]))
        }
        Field::Pet => {
            let (k, n) = p.pet?;
            let mut f = mk(format!("{}の「{}」", PET_KINDS[k], PET_NAMES[n].0));
            f.key = PET_NAMES[n].0.to_string();
            f
        }
        Field::Face => mk("顔写真".to_string()),
        Field::IdDoc => mk(if p.age_at(now) >= 18 {
            "運転免許証の画像（氏名・住所・生年月日・顔写真入り）".to_string()
        } else {
            "学生証の画像（氏名・学校名・顔写真入り）".to_string()
        }),
        Field::Income => mk(format!("年収 約{}万円", p.income?)),
        Field::Assets => mk(p.assets.clone()?),
        Field::Card => {
            let c = p.card.clone()?;
            mk(if variant == 1 {
                format!("カード番号 全16桁（末尾 {}）・有効期限 {}・セキュリティコード", c, p.card_exp)
            } else {
                format!("下4桁 {}（有効期限 {}）", c, p.card_exp)
            })
        }
        Field::Password => {
            let mut f = mk(match variant {
                1 => format!("{}（偽ページに入力させた）", p.password),
                2 => "（再設定して乗っ取り済み）".to_string(),
                _ => p.password.clone(),
            });
            if variant != 2 {
                f.aux = hash_str(&p.password) as i64;
                f.key = p.password.clone();
            }
            f
        }
        Field::History => {
            let h = CATEGORIES[(variant as usize).min(CATEGORIES.len() - 1)].history;
            let i = (hash01(&[w.seed, 0x4157, id as u64, variant as u64]) * h.len() as f64) as usize;
            mk(h[i.min(h.len() - 1)].to_string())
        }
        Field::Health => mk(HEALTH[p.health?].to_string()),
    };
    if f.pref.is_none() && matches!(field, Field::Pref | Field::City | Field::Station | Field::Postal | Field::Town | Field::Address) {
        f.pref = Some(a.pref);
    }
    Some(f)
}
