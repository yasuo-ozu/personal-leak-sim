//! 流出インシデントの生成と、ある人物が流出対象に含まれるかの判定。

use rand::RngExt;

use crate::data::*;
use crate::facts;
use crate::model::*;
use crate::seed::{hash01, Rng, RngHelpers};
use crate::world::World;

/// あるアイドルグループのファンが人口に占める割合
const FAN_SHARE: f64 = 0.004;
pub const CAUSE_SCRAPE: usize = CAUSES.len();

pub fn cause_label(i: usize) -> &'static str {
    if i == CAUSE_SCRAPE {
        "公開機能の悪用による大量収集（スクレイピング）"
    } else {
        CAUSES[i].0
    }
}

fn pref_share(pref: usize) -> f64 {
    let total: f64 = PREFS.iter().map(|p| p.3).sum();
    PREFS[pref].3 / total
}

pub fn generate(w: &World, id: u32, announce: i32, persona: u32, r: &mut Rng) -> Incident {
    let weights: Vec<f64> = CATEGORIES.iter().map(|c| c.freq).collect();
    let cat = r.weighted(&weights);
    let c = &CATEGORIES[cat];
    let me = w.p(persona);
    let my_pref = w.addr(persona).pref;

    let mut pref = None;
    let mut idol = None;
    let mut platform = 0u8;
    let mut share = 1.0;
    let local = match c.special {
        Special::Local => true,
        Special::MaybeLocal => r.chance(0.5),
        _ => false,
    };
    if local {
        let pr = if r.chance(0.25) { my_pref } else { World::random_pref(r) };
        pref = Some(pr);
        share = pref_share(pr);
    }
    match c.special {
        Special::Fan => {
            let g = match me.idol {
                Some((g, _)) if r.chance(0.35) => g,
                _ => r.random_range(0..IDOL_GROUPS.len()),
            };
            idol = Some(g);
            share = FAN_SHARE;
        }
        Special::YoungParent => share = 0.12,
        Special::Sns => {
            platform = r.weighted(&[1.0, 1.2, 0.8]) as u8;
            share = 0.4;
        }
        Special::Univ => share = 0.022,
        _ => {}
    }

    let cause = match c.special {
        Special::Univ => CAUSE_MISSEND,
        Special::Sns => CAUSE_SCRAPE,
        _ => {
            let cw: Vec<f64> = CAUSES.iter().map(|c| c.2).collect();
            r.weighted(&cw)
        }
    };
    let mut pct = r.log_uniform(c.scale.0, c.scale.1);
    if r.chance(0.05) {
        pct = (pct * 3.0).min(25.0);
    }
    let mut fields: Vec<Field> = c.fields.iter().filter(|(_, p)| r.chance(*p)).map(|(f, _)| *f).collect();
    if cause == CAUSE_MISSEND {
        pct = pct.min(0.003);
        fields.retain(|f| matches!(f, Field::Email | Field::Name | Field::School));
    }
    if fields.is_empty() {
        fields.push(c.fields[0].0);
    }
    let female = (c.female + r.normal() * 0.06).clamp(0.02, 0.98);
    let mut age = c.age;
    for a in age.iter_mut() {
        *a *= (r.normal() * 0.25).exp();
    }
    let norm: f64 = AGE_SHARE.iter().zip(age.iter()).map(|(s, a)| s * a).sum::<f64>() * share;

    let pn = *r.pick(COMPANY_NAME_P);
    let sn = *r.pick(c.suffixes);
    let (company, service) = match c.label {
        "自治体（業務委託先）" => {
            let city = World::random_city(pref.unwrap_or(my_pref), r);
            (format!("{}役所", CITIES[city].1), format!("住民情報システム（業務委託先: 株式会社{}情報サービス）", pn))
        }
        "病院・クリニック" => (format!("医療法人{}会 {}{}", pn, r.pick(TOWN_A), sn), "電子カルテシステム".to_string()),
        "ファンクラブ" => {
            let g = idol.unwrap_or(0);
            (format!("{} オフィシャルファンクラブ", IDOL_GROUPS[g].0), format!("会員管理システム（運営: 株式会社{}エンタテインメント）", pn))
        }
        "大学（メール誤送信）" => (r.pick(UNIVS).0.to_string(), "学生向け一斉メール".to_string()),
        "SNS（スクレイピング）" => {
            let pl = PLATFORMS[platform as usize];
            (pl.0.to_string(), pl.2.to_string())
        }
        _ => (format!("株式会社{}{}", pn, sn), format!("{}「{}{}」", c.label, pn, sn)),
    };
    let occur = announce - r.random_range(14..150);
    let circ = if r.chance(if cause == CAUSE_SCRAPE { 0.9 } else { CAUSES[cause].1 }) {
        Some(announce + r.random_range(3..90))
    } else {
        None
    };
    Incident {
        id,
        company,
        service,
        cat,
        cause,
        occur,
        announce,
        circ,
        pct,
        count: (pct / 100.0 * JAPAN_POP) as u64,
        fields,
        female,
        age,
        norm,
        pref,
        idol,
        platform,
        plain_pw: r.chance(0.15),
        persona_hit: false,
    }
}

/// 顧客層に対するこの人物の重み（0なら顧客ではない）
pub fn weight(w: &World, inc: &Incident, id: u32) -> f64 {
    let p = w.p(id);
    if let Some(pr) = inc.pref {
        if w.addr(id).pref != pr {
            return 0.0;
        }
    }
    let ok = match CATEGORIES[inc.cat].special {
        Special::Fan => p.kind != Kind::Idol && p.idol.map(|x| x.0) == inc.idol,
        Special::YoungParent => w.has_young_child(id, inc.announce),
        Special::Sns => p.account(inc.platform).is_some(),
        Special::Univ => p.occupation == "大学生",
        _ => true,
    };
    if !ok {
        return 0.0;
    }
    let band = (p.age_at(inc.occur).max(0) / 10).min(8) as usize;
    let g = if p.gender == Gender::Female { inc.female * 2.0 } else { (1.0 - inc.female) * 2.0 };
    inc.age[band] * g
}

pub fn p_leak(w: &World, inc: &Incident, id: u32) -> f64 {
    if inc.norm <= 0.0 {
        return 0.0;
    }
    (inc.pct / 100.0 * weight(w, inc, id) / inc.norm).min(0.9)
}

pub fn leaked(w: &World, inc: &Incident, id: u32) -> bool {
    let p = w.p(id);
    if p.phantom_inc == Some(inc.id) {
        return true;
    }
    if p.birth > inc.occur {
        return false;
    }
    hash01(&[w.seed, 0x1EA4, inc.id as u64, id as u64]) < p_leak(w, inc, id)
}

fn routine_for(w: &World, inc: &Incident, id: u32) -> String {
    let label = CATEGORIES[inc.cat].label;
    let st = &w.station_of(id).name;
    if label.contains("ジム") {
        format!("入館履歴: 平日{}時ごろに来館（{}前店）", 18 + (id % 4), st.trim_end_matches('駅'))
    } else if label.contains("電力") {
        "電力使用量の推移から、平日の日中は不在と推測できる".to_string()
    } else if label.contains("配送") {
        "再配達依頼の履歴から、平日の日中は不在が多い".to_string()
    } else {
        w.p(id).routine.clone()
    }
}

/// 流出データに含まれるこの人物の1行
pub fn record(w: &World, inc: &Incident, id: u32) -> Vec<FactInit> {
    let p = w.p(id);
    let c = &CATEGORIES[inc.cat];
    let mut v = Vec::new();
    for &f in &inc.fields {
        let item = match f {
            Field::Email => {
                if c.special == Special::Univ {
                    facts::value(w, id, f, 2, inc.announce)
                } else if p.emails.len() > 1 && hash01(&[w.seed, 0xE3A1, inc.id as u64, id as u64]) < 0.5 {
                    facts::value(w, id, f, 4, inc.announce)
                } else {
                    facts::value(w, id, f, 0, inc.announce)
                }
            }
            Field::Handle => {
                if c.special == Special::Sns {
                    facts::value(w, id, f, inc.platform, inc.announce)
                } else {
                    let mut fi = FactInit::new(f, format!("ゲームID: {}", p.base_handle), id);
                    fi.key = p.base_handle.clone();
                    fi.aux = 99;
                    Some(fi)
                }
            }
            Field::History => facts::value(w, id, f, inc.cat as u8, inc.announce),
            Field::Password => {
                let mut fi = FactInit::new(f, String::new(), id);
                if inc.plain_pw {
                    fi.value = format!("{}（平文で保存されていた）", p.password);
                } else if p.pass_weak {
                    fi.value = format!("{}（ハッシュ値から解読）", p.password);
                } else {
                    fi.value = "（ハッシュ値・解読できず）".to_string();
                }
                if inc.plain_pw || p.pass_weak {
                    fi.aux = crate::seed::hash_str(&p.password) as i64;
                    fi.key = p.password.clone();
                }
                Some(fi)
            }
            Field::Face => Some(FactInit::new(f, format!("顔写真データ（{}に登録）", inc.company), id)),
            Field::Routine => Some(FactInit::new(f, routine_for(w, inc, id), id)),
            Field::Idol => facts::value(w, id, f, 0, inc.announce).or_else(|| {
                inc.idol.map(|g| FactInit::new(f, format!("{} 会員", IDOL_GROUPS[g].0), id))
            }),
            _ => facts::value(w, id, f, 0, inc.announce),
        };
        if let Some(i) = item {
            v.push(i);
        }
    }
    v
}

pub fn fmt_count(n: u64) -> String {
    let n = n.max(1) as f64;
    if n >= 1e8 {
        format!("約{:.1}億", n / 1e8)
    } else if n >= 1e7 {
        format!("約{}万", group(((n / 1e4 / 10.0).round() * 10.0) as u64))
    } else if n >= 1e4 {
        format!("約{}万", group((n / 1e4).round() as u64))
    } else if n >= 1000.0 {
        format!("約{}", group(((n / 100.0).round() * 100.0) as u64))
    } else {
        format!("約{}", group(n.round() as u64))
    }
}

pub fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn fmt_pct(pct: f64) -> String {
    let s = if pct >= 1.0 {
        format!("{:.1}%", pct)
    } else if pct >= 0.1 {
        format!("{:.2}%", pct)
    } else if pct >= 0.01 {
        format!("{:.3}%", pct)
    } else {
        format!("{:.4}%", pct)
    };
    let ratio = 100.0 / pct.max(1e-9);
    let r = if ratio < 10.0 {
        format!("約{:.1}人に1人", ratio)
    } else {
        format!("約{}人に1人", group(ratio.round() as u64))
    };
    format!("日本人の{}（{}）", s, r)
}

pub fn severity(inc: &Incident) -> u8 {
    let sens = inc.fields.iter().map(|f| f.risk()).sum::<u32>();
    let size = if inc.pct >= 1.0 {
        3
    } else if inc.pct >= 0.1 {
        2
    } else {
        1
    };
    (size + if sens >= 60 { 2 } else if sens >= 30 { 1 } else { 0 }).min(5) as u8
}
