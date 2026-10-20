//! プレイヤーのペルソナと「自分の持っている情報」の生成。

use rand::RngExt;
use serde::Serialize;

use crate::data::*;
use crate::date;
use crate::model::*;
use crate::seed::{Rng, RngHelpers};
use crate::world::{Place, Spec, World};

#[derive(Serialize)]
pub struct PersonaInfo {
    pub key: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub place: &'static str,
    pub summary: &'static str,
}

pub const PERSONAS: &[PersonaInfo] = &[
    PersonaInfo {
        key: "student",
        title: "高校2年生（女子）",
        icon: "🎒",
        place: "神奈川県横浜市港北区",
        summary: "クラスの連絡網、部活のLINE、Snapgramの相互フォロー、中学の卒業アルバム、推しアイドルの公式アカウント。",
    },
    PersonaInfo {
        key: "office",
        title: "会社員（34歳・男性）",
        icon: "💼",
        place: "東京都世田谷区",
        summary: "名刺フォルダ、届いた年賀状、高校の同窓会名簿、社内チャット、Chatterのフォロー。",
    },
    PersonaInfo {
        key: "parent",
        title: "子育て中の主婦（41歳）",
        icon: "🏠",
        place: "埼玉県さいたま市大宮区",
        summary: "回覧板、PTAの連絡網、ママ友のLINE、スイミングスクールの名簿、Snapgram。",
    },
    PersonaInfo {
        key: "college",
        title: "大学生（20歳・男性）",
        icon: "🎓",
        place: "京都府京都市左京区",
        summary: "サークル名簿、バイト先のシフト表、高校の同級生LINE、Chatterの相互フォロー、推しの配信者。",
    },
    PersonaInfo {
        key: "senior",
        title: "定年退職者（68歳・男性）",
        icon: "🍵",
        place: "宮城県仙台市青葉区",
        summary: "町内会名簿、年賀状、元職場のOB会名簿、家族と孫のSnapgram。",
    },
];

pub enum Known {
    Auto(Field, u8),
    Custom(Field, String),
}

pub struct Entry {
    pub person: u32,
    pub relation: String,
    pub known: Vec<Known>,
}

pub struct Source {
    pub name: String,
    pub icon: &'static str,
    pub desc: String,
    pub entries: Vec<Entry>,
}

use Field::{Address, Age, Alias, BirthDate, BirthMd, Email, Face, Family, Handle, Hobby, HomePhone, Kana, Mobile, Name, OldAddress, Postal, School, Station, Workplace};

fn follow(w: &mut World, id: u32, pf: u8) {
    if let Some(a) = w.pm(id).accounts.iter_mut().find(|a| a.platform == pf) {
        a.persona_follows = true;
    }
}

/// specs: (項目, variant, 知っている確率)
fn entry(w: &mut World, r: &mut Rng, me: u32, id: u32, rel: &str, specs: &[(Field, u8, f64)]) -> Entry {
    let mut known = Vec::new();
    for &(f, v, p) in specs {
        if !r.chance(p) {
            continue;
        }
        if f == Handle {
            w.add_account(id, v, r);
            follow(w, id, v);
        }
        known.push(Known::Auto(f, v));
    }
    w.link(me, id);
    Entry { person: id, relation: rel.to_string(), known }
}

fn link_group(w: &mut World, ids: &[u32], r: &mut Rng, k: usize) {
    if ids.is_empty() {
        return;
    }
    for &a in ids {
        for _ in 0..k {
            let b = ids[r.random_range(0..ids.len())];
            w.link(a, b);
        }
    }
}

fn spawn_parents(w: &mut World, hh: usize, child_age: i32, sur: usize, r: &mut Rng) {
    if r.chance(0.85) {
        let fa = child_age + r.random_range(26..=36);
        let mut s = Spec::new((fa, fa), Place::Household(hh)).gender(Gender::Male).kind(Kind::Family);
        s.sur = Some(sur);
        w.spawn(s, r);
    }
    let ma = child_age + r.random_range(24..=34);
    let mut s = Spec::new((ma, ma), Place::Household(hh)).gender(Gender::Female).kind(Kind::Family);
    s.sur = Some(sur);
    s.role = Role::Spouse;
    w.spawn(s, r);
}

fn set_high_school(w: &mut World, id: u32, hs: &str) {
    let p = w.pm(id);
    let old = p.past_schools.get(2).cloned();
    if let Some(x) = p.past_schools.get_mut(2) {
        *x = hs.to_string();
    }
    if old.is_some() && p.school == old {
        p.school = Some(hs.to_string());
    }
}

fn spawn_idol(w: &mut World, r: &mut Rng, g: usize, m: usize, gender: Gender, ages: (i32, i32)) -> u32 {
    let pref = *r.pick(&[12usize, 12, 13, 10, 11]);
    let mut s = Spec::new(ages, Place::Pref(pref)).gender(gender).kind(Kind::Idol);
    s.occupation = Some(if g == 4 { "配信者" } else { "アイドル" });
    s.workplace = Some((format!("株式会社{}プロダクション", r.pick(COMPANY_NAME_P)), String::new()));
    s.idol = Some((g, m));
    s.home_phone = Some(false);
    let id = w.spawn(s, r);
    w.pm(id).stage = Some(IDOL_GROUPS[g].1[m].to_string());
    w.pm(id).overshare = 0.35;
    id
}

fn metro(r: &mut Rng) -> Place {
    Place::Pref(*r.pick(&[12usize, 12, 12, 13, 10, 11]))
}

pub fn build(key: &str, w: &mut World, r: &mut Rng) -> Option<(u32, Vec<Source>)> {
    Some(match key {
        "student" => student(w, r),
        "office" => office(w, r),
        "parent" => parent(w, r),
        "college" => college(w, r),
        "senior" => senior(w, r),
        _ => return None,
    })
}

fn student(w: &mut World, r: &mut Rng) -> (u32, Vec<Source>) {
    let city = city_index("横浜市港北区");
    let cohort = date::year(w.start) - 17;
    let school = w.high_school(13, r);
    let mut s = Spec::new((16, 17), Place::City(city as usize)).gender(Gender::Female).cohort(cohort).school(&school).kind(Kind::Persona);
    s.home_phone = Some(true);
    s.idol = Some((0, 0));
    let me = w.spawn(s, r);
    w.add_family(me, r);
    w.add_account(me, 1, r);
    let my_middle = w.p(me).past_schools.get(1).cloned().unwrap_or_default();
    let near = [city, city_index("横浜市青葉区"), city_index("川崎市中原区"), city_index("横浜市中区")].map(|c| c as usize);
    let mut sources = Vec::new();

    let mut classmates = Vec::new();
    for _ in 0..30 {
        let c = near[r.weighted(&[0.6, 0.18, 0.14, 0.08])];
        let mut s = Spec::new((16, 17), Place::City(c)).cohort(cohort).school(&school);
        s.home_phone = Some(true);
        let id = w.spawn(s, r);
        w.add_family(id, r);
        classmates.push(id);
    }
    link_group(w, &classmates, r, 4);
    let entries = classmates
        .iter()
        .map(|&id| entry(w, r, me, id, "クラスメイト", &[(Name, 0, 1.0), (HomePhone, 0, 1.0), (Age, 1, 1.0)]))
        .collect();
    sources.push(Source {
        name: "2年3組 クラス連絡網".into(),
        icon: "📞",
        desc: "クラス全員の氏名と自宅の電話番号が載った紙の連絡網。全員が同じ学年。".into(),
        entries,
    });

    let mut club: Vec<u32> = classmates.iter().copied().filter(|&id| w.p(id).gender == Gender::Female).take(4).collect();
    for _ in 0..6 {
        let c = near[r.weighted(&[0.6, 0.2, 0.2, 0.0])];
        let s = Spec::new((15, 18), Place::City(c)).gender(Gender::Female).cohort(cohort + r.random_range(-1..=1)).school(&school);
        let id = w.spawn(s, r);
        w.add_family(id, r);
        club.push(id);
    }
    link_group(w, &club, r, 3);
    let entries = club
        .iter()
        .map(|&id| entry(w, r, me, id, "バレー部の仲間", &[(Name, 0, 1.0), (Alias, 0, 1.0), (BirthMd, 0, 0.6), (Handle, 1, 0.7)]))
        .collect();
    sources.push(Source {
        name: "女子バレー部 LINEグループ".into(),
        icon: "💬",
        desc: "部員の表示名と本名。LINEの誕生日通知で誕生日もわかる。".into(),
        entries,
    });

    let mut alumni = Vec::new();
    for _ in 0..22 {
        let id = w.spawn(Spec::new((16, 17), Place::City(city as usize)).cohort(cohort), r);
        w.add_family(id, r);
        if let Some(m) = w.pm(id).past_schools.get_mut(1) {
            *m = my_middle.clone();
        }
        alumni.push(id);
    }
    link_group(w, &alumni, r, 3);
    let entries = alumni
        .iter()
        .map(|&id| {
            let mut e = entry(w, r, me, id, "中学の同級生", &[(Name, 0, 1.0), (Kana, 0, 1.0), (School, 2, 1.0)]);
            e.known.push(Known::Custom(Face, "卒業アルバムの顔写真（中学3年時）".into()));
            e
        })
        .collect();
    sources.push(Source {
        name: "中学の卒業アルバム".into(),
        icon: "📕",
        desc: "同じ中学を卒業した同級生の氏名・ふりがなと顔写真。".into(),
        entries,
    });

    let mut snap: Vec<(u32, &str)> = Vec::new();
    for &id in alumni.iter().take(8) {
        snap.push((id, "中学の同級生"));
    }
    for &id in classmates.iter().skip(10).take(5) {
        snap.push((id, "クラスメイト"));
    }
    for _ in 0..7 {
        let place = if r.chance(0.5) { Place::Pref(13) } else { Place::Anywhere };
        let id = w.spawn(Spec::new((15, 19), place).cohort(cohort + r.random_range(-1..=1)), r);
        w.add_family(id, r);
        snap.push((id, "ネットの友達"));
    }
    let entries = snap.iter().map(|&(id, rel)| entry(w, r, me, id, rel, &[(Handle, 1, 1.0), (Alias, 0, 1.0)])).collect();
    sources.push(Source {
        name: "Snapgram 相互フォロー".into(),
        icon: "📷",
        desc: "相互フォローしているので、鍵アカウントでも投稿が見られる。".into(),
        entries,
    });

    let mut idols = vec![(spawn_idol(w, r, 0, 0, Gender::Female, (19, 23)), "推し（ルミナス☆ガールズ）")];
    if r.chance(0.6) {
        idols.push((spawn_idol(w, r, 1, 0, Gender::Male, (20, 25)), "推し（STELLAR BOYS）"));
    }
    let entries = idols
        .iter()
        .map(|&(id, rel)| entry(w, r, me, id, rel, &[(Alias, 0, 1.0), (Handle, 0, 1.0), (Handle, 1, 1.0), (BirthMd, 0, 1.0)]))
        .collect();
    sources.push(Source {
        name: "推しの公式アカウント".into(),
        icon: "⭐",
        desc: "芸名と公式SNS、公式プロフィールの誕生日だけを知っている。".into(),
        entries,
    });
    (me, sources)
}

fn office(w: &mut World, r: &mut Rng) -> (u32, Vec<Source>) {
    let city = city_index("世田谷区") as usize;
    let home = city_index("静岡市葵区") as usize;
    let company = ("株式会社アオバシステムズ".to_string(), "aoba-systems.example".to_string());
    let mut s = Spec::new((34, 34), Place::City(city)).gender(Gender::Male).kind(Kind::Persona);
    s.occupation = Some("会社員");
    s.workplace = Some(company.clone());
    let me = w.spawn(s, r);
    w.add_family(me, r);
    w.add_account(me, 0, r);
    let cohort = date::cohort_year(w.p(me).birth);
    let hs = w.high_school(21, r);
    let hh = w.new_household(&Place::City(home), 1.0, r);
    let sur = w.p(me).sur;
    spawn_parents(w, hh, 34, sur, r);
    w.pm(me).hometown = Some(hh);
    set_high_school(w, me, &hs);
    let mut sources = Vec::new();

    let mut cards = Vec::new();
    for _ in 0..35 {
        let place = if r.chance(0.95) { metro(r) } else { Place::Anywhere };
        let mut s = Spec::new((25, 60), place);
        s.occupation = Some("会社員");
        let id = w.spawn(s, r);
        w.add_family(id, r);
        cards.push(id);
    }
    let entries = cards
        .iter()
        .map(|&id| entry(w, r, me, id, "取引先", &[(Name, 0, 1.0), (Kana, 0, 0.7), (Workplace, 0, 1.0), (Email, 1, 1.0), (Mobile, 0, 0.4)]))
        .collect();
    sources.push(Source {
        name: "名刺フォルダ".into(),
        icon: "📇",
        desc: "仕事で交換した名刺。氏名・会社名・仕事用メール、たまに携帯番号。".into(),
        entries,
    });

    let mut alumni = Vec::new();
    for _ in 0..30 {
        let hh = w.new_household(&Place::City(home), 0.9, r);
        let mode = r.weighted(&[0.2, 0.25, 0.4, 0.15]);
        let place = match mode {
            0 => Place::Household(hh),
            1 => Place::City(home),
            2 => metro(r),
            _ => Place::Anywhere,
        };
        let id = w.spawn(Spec::new((34, 34), place).cohort(cohort), r);
        set_high_school(w, id, &hs);
        let sur = w.p(id).sur;
        spawn_parents(w, hh, 34, sur, r);
        if mode == 0 {
            w.pm(id).role = Role::Child;
        } else {
            w.pm(id).hometown = Some(hh);
            w.add_family(id, r);
        }
        alumni.push(id);
    }
    link_group(w, &alumni, r, 3);
    let entries = alumni
        .iter()
        .map(|&id| {
            if w.p(id).hometown.is_some() {
                entry(w, r, me, id, "高校の同級生", &[(Name, 0, 1.0), (Kana, 0, 1.0), (School, 3, 1.0), (OldAddress, 0, 0.9), (HomePhone, 1, 0.9)])
            } else {
                entry(w, r, me, id, "高校の同級生", &[(Name, 0, 1.0), (Kana, 0, 1.0), (School, 3, 1.0), (Address, 0, 0.9), (HomePhone, 0, 0.9)])
            }
        })
        .collect();
    sources.push(Source {
        name: "高校の同窓会名簿".into(),
        icon: "📒",
        desc: "卒業時に配られた名簿。住所と電話番号は当時の実家のもの。".into(),
        entries,
    });

    let mut nenga: Vec<(u32, &str)> = alumni.iter().filter(|&&id| w.p(id).hometown.is_some()).take(5).map(|&id| (id, "高校の同級生")).collect();
    for _ in 0..4 {
        let place = match r.weighted(&[0.5, 0.3, 0.2]) {
            0 => Place::City(home),
            1 => Place::Pref(12),
            _ => Place::Anywhere,
        };
        let mut s = Spec::new((30, 70), place);
        if r.random_bool(0.5) {
            s.sur = Some(sur);
        }
        let id = w.spawn(s, r);
        w.add_family(id, r);
        nenga.push((id, "親戚"));
    }
    for _ in 0..6 {
        let id = w.spawn(Spec::new((33, 35), Place::Anywhere), r);
        w.add_family(id, r);
        nenga.push((id, "大学の友人"));
    }
    let entries = nenga
        .iter()
        .map(|&(id, rel)| entry(w, r, me, id, rel, &[(Name, 0, 1.0), (Address, 0, 1.0), (Postal, 0, 1.0), (Family, 0, 0.5)]))
        .collect();
    sources.push(Source {
        name: "今年届いた年賀状".into(),
        icon: "🎍",
        desc: "差出人の住所・郵便番号と、連名で書かれた家族の名前。".into(),
        entries,
    });

    let mut coworkers = Vec::new();
    for _ in 0..25 {
        let mut s = Spec::new((22, 60), metro(r));
        s.occupation = Some("会社員");
        s.workplace = Some(company.clone());
        let id = w.spawn(s, r);
        w.add_family(id, r);
        coworkers.push(id);
    }
    link_group(w, &coworkers, r, 4);
    let entries = coworkers
        .iter()
        .map(|&id| entry(w, r, me, id, "同僚", &[(Name, 0, 1.0), (Workplace, 1, 1.0), (Email, 1, 1.0), (Alias, 0, 0.5), (Hobby, 0, 0.3)]))
        .collect();
    sources.push(Source {
        name: "社内チャット".into(),
        icon: "🗨",
        desc: "同じ会社の社員の氏名・部署・社内メールアドレス。".into(),
        entries,
    });

    let mut chat: Vec<(u32, &str)> = coworkers.iter().take(4).map(|&id| (id, "同僚")).collect();
    chat.extend(alumni.iter().skip(5).take(3).map(|&id| (id, "高校の同級生")));
    for _ in 0..8 {
        let id = w.spawn(Spec::new((25, 45), Place::Anywhere), r);
        w.add_family(id, r);
        chat.push((id, "ネットの知り合い"));
    }
    let entries = chat.iter().map(|&(id, rel)| entry(w, r, me, id, rel, &[(Handle, 0, 1.0), (Alias, 0, 1.0)])).collect();
    sources.push(Source {
        name: "Chatter 相互フォロー".into(),
        icon: "🐦",
        desc: "相互フォローしているアカウント。鍵アカウントでも投稿が見られる。".into(),
        entries,
    });
    (me, sources)
}

fn parent(w: &mut World, r: &mut Rng) -> (u32, Vec<Source>) {
    let city = city_index("さいたま市大宮区") as usize;
    let town = w.random_town(city, r);
    let chome = 2u8;
    let mut s = Spec::new((43, 43), Place::Town(town, Some(chome))).gender(Gender::Male).kind(Kind::Family);
    s.home_phone = Some(true);
    s.occupation = Some("会社員");
    let husband = w.spawn(s, r);
    let hh = w.p(husband).household;
    let sur = w.p(husband).sur;
    let mut s = Spec::new((41, 41), Place::Household(hh)).gender(Gender::Female).kind(Kind::Persona);
    s.sur = Some(sur);
    s.role = Role::Spouse;
    s.occupation = Some("専業主婦");
    let me = w.spawn(s, r);
    let child_cohort = date::year(w.start) - 9;
    let mut s = Spec::new((8, 9), Place::Household(hh)).cohort(child_cohort).kind(Kind::Family);
    s.sur = Some(sur);
    s.role = Role::Child;
    let child = w.spawn(s, r);
    let mut s = Spec::new((5, 6), Place::Household(hh)).kind(Kind::Family);
    s.sur = Some(sur);
    s.role = Role::Child;
    let child2 = w.spawn(s, r);
    for x in [husband, child, child2] {
        w.link(me, x);
    }
    w.add_account(me, 1, r);
    let child_school = w.p(child).school.clone().unwrap_or_default();
    let tname = w.towns[town].name.clone();
    let mut sources = Vec::new();

    let mut nb = Vec::new();
    for _ in 0..10 {
        let mut s = Spec::new((30, 85), Place::Town(town, Some(chome)));
        s.home_phone = Some(r.chance(0.6));
        let id = w.spawn(s, r);
        w.add_family(id, r);
        nb.push(id);
    }
    link_group(w, &nb, r, 2);
    let entries = nb
        .iter()
        .map(|&id| entry(w, r, me, id, "ご近所（同じ班）", &[(Name, 0, 1.0), (Address, 0, 1.0), (HomePhone, 0, 0.5)]))
        .collect();
    sources.push(Source {
        name: format!("回覧板（{}{}丁目 第2班）", tname, chome),
        icon: "📋",
        desc: "班の世帯主の名前と番地が回覧順に並んでいる。".into(),
        entries,
    });

    let mut pta = Vec::new();
    for _ in 0..25 {
        let place = if r.chance(0.45) { Place::Town(town, None) } else { Place::City(city) };
        let kid = w.spawn(Spec::new((8, 9), place).cohort(child_cohort).school(&child_school), r);
        w.add_family(kid, r);
        let h = w.p(kid).household;
        let parents: Vec<u32> = w.households[h].members.iter().copied().filter(|&m| w.p(m).role != Role::Child).collect();
        let mom = parents.iter().copied().find(|&m| w.p(m).gender == Gender::Female);
        let parent = match (mom, r.chance(0.8)) {
            (Some(m), true) => m,
            _ => match parents.first() {
                Some(&p) => p,
                None => continue,
            },
        };
        pta.push((parent, kid));
    }
    let pids: Vec<u32> = pta.iter().map(|x| x.0).collect();
    link_group(w, &pids, r, 3);
    let entries = pta
        .iter()
        .map(|&(pid, kid)| {
            let mut e = entry(w, r, me, pid, "PTA（3年2組の保護者）", &[(Name, 0, 1.0), (Mobile, 0, 1.0)]);
            e.known.push(Known::Custom(Family, format!("子: {}（3年2組）", w.p(kid).given.0)));
            e
        })
        .collect();
    sources.push(Source {
        name: "PTA連絡網（3年2組）".into(),
        icon: "📞",
        desc: "子どものクラスの保護者の名前と携帯番号、子どもの名前。".into(),
        entries,
    });

    let mut moms: Vec<u32> = pta.iter().map(|x| x.0).filter(|&id| w.p(id).gender == Gender::Female).take(5).collect();
    for _ in 0..2 {
        let id = w.spawn(Spec::new((35, 45), Place::City(city)).gender(Gender::Female), r);
        w.add_family(id, r);
        moms.push(id);
    }
    let entries = moms
        .iter()
        .map(|&id| entry(w, r, me, id, "ママ友", &[(Alias, 0, 1.0), (Name, 0, 0.6), (Handle, 1, 0.8), (Station, 0, 0.4)]))
        .collect();
    sources.push(Source {
        name: "ママ友 LINEグループ".into(),
        icon: "💬",
        desc: "表示名と、ときどき話題に出る最寄り駅。Snapgramも教え合っている。".into(),
        entries,
    });

    let mut swim = Vec::new();
    for _ in 0..12 {
        let kid = w.spawn(Spec::new((5, 10), Place::City(city)), r);
        w.add_family(kid, r);
        let h = w.p(kid).household;
        if let Some(&pid) = w.households[h].members.iter().find(|&&m| w.p(m).role != Role::Child) {
            swim.push((pid, kid));
        }
    }
    let entries = swim
        .iter()
        .map(|&(pid, kid)| {
            let mut e = entry(w, r, me, pid, "スイミングの保護者", &[(Name, 0, 1.0), (Mobile, 0, 0.5)]);
            e.known.push(Known::Custom(Family, format!("子: {}（スイミング{}級）", w.p(kid).given.0, r.random_range(5..=15))));
            e
        })
        .collect();
    sources.push(Source {
        name: "スイミングスクールの名簿".into(),
        icon: "🏊",
        desc: "送迎当番のための保護者名簿。".into(),
        entries,
    });

    let mut sg: Vec<(u32, &str)> = moms.iter().take(4).map(|&id| (id, "ママ友")).collect();
    for _ in 0..8 {
        let place = if r.chance(0.6) { Place::Pref(10) } else { Place::Anywhere };
        let id = w.spawn(Spec::new((35, 45), place).gender(Gender::Female), r);
        w.add_family(id, r);
        sg.push((id, "昔からの友達"));
    }
    let entries = sg.iter().map(|&(id, rel)| entry(w, r, me, id, rel, &[(Handle, 1, 1.0), (Alias, 0, 1.0)])).collect();
    sources.push(Source {
        name: "Snapgram 相互フォロー".into(),
        icon: "📷",
        desc: "相互フォローしているので鍵アカウントでも投稿が見られる。".into(),
        entries,
    });
    (me, sources)
}

fn college(w: &mut World, r: &mut Rng) -> (u32, Vec<Source>) {
    let city = city_index("京都市左京区") as usize;
    let home = city_index("広島市中区") as usize;
    let mut s = Spec::new((20, 20), Place::City(city)).gender(Gender::Male).kind(Kind::Persona);
    s.univ = Some(2);
    s.school = Some("京洛大学 工学部".into());
    s.occupation = Some("大学生");
    s.home_phone = Some(false);
    let me = w.spawn(s, r);
    w.add_account(me, 0, r);
    w.add_account(me, 1, r);
    let sur = w.p(me).sur;
    let hh = w.new_household(&Place::City(home), 0.8, r);
    spawn_parents(w, hh, 20, sur, r);
    w.pm(me).hometown = Some(hh);
    let hs = w.high_school(33, r);
    set_high_school(w, me, &hs);
    let cohort = date::cohort_year(w.p(me).birth);
    let near = ["京都市左京区", "京都市中京区", "京都市伏見区", "宇治市", "大津市", "大阪市北区"].map(|c| city_index(c) as usize);
    let mut sources = Vec::new();

    let mut circle = Vec::new();
    for _ in 0..30 {
        let c = near[r.weighted(&[0.4, 0.2, 0.15, 0.1, 0.1, 0.05])];
        let mut s = Spec::new((18, 22), Place::City(c));
        s.univ = Some(2);
        s.occupation = Some("大学生");
        s.school = Some(format!("京洛大学 {}", r.pick(&["工学部", "文学部", "経済学部", "法学部", "理学部"])));
        s.home_phone = Some(false);
        let id = w.spawn(s, r);
        if r.chance(0.6) {
            let hh = w.new_household(&Place::Anywhere, 0.8, r);
            let (age, sur) = (w.p(id).age_at(w.start), w.p(id).sur);
            spawn_parents(w, hh, age, sur, r);
            w.pm(id).hometown = Some(hh);
        } else {
            w.add_family(id, r);
        }
        circle.push(id);
    }
    link_group(w, &circle, r, 5);
    let entries = circle
        .iter()
        .map(|&id| {
            entry(w, r, me, id, "サークルの仲間", &[(Name, 0, 1.0), (Kana, 0, 1.0), (School, 0, 1.0), (Email, 2, 1.0), (Mobile, 0, 0.8), (BirthMd, 0, 0.4)])
        })
        .collect();
    sources.push(Source {
        name: "テニスサークル部員名簿".into(),
        icon: "🎾",
        desc: "新歓で作った名簿。学部・大学のメール・携帯番号・誕生日欄がある。".into(),
        entries,
    });

    let mut baito = Vec::new();
    for _ in 0..12 {
        let id = w.spawn(Spec::new((17, 45), Place::City(near[r.random_range(0..2)])), r);
        w.add_family(id, r);
        baito.push(id);
    }
    let entries = baito
        .iter()
        .map(|&id| entry(w, r, me, id, "バイト仲間", &[(Name, 0, 1.0), (Mobile, 0, 0.6), (Alias, 0, 0.5)]))
        .collect();
    sources.push(Source {
        name: "バイト先のシフト表（カフェ）".into(),
        icon: "☕",
        desc: "スタッフの名前と緊急連絡用の携帯番号が書かれている。".into(),
        entries,
    });

    let mut hsmates = Vec::new();
    for _ in 0..20 {
        let id = if r.chance(0.5) {
            let id = w.spawn(Spec::new((20, 20), Place::City(home)).cohort(cohort), r);
            w.add_family(id, r);
            id
        } else {
            let hh = w.new_household(&Place::City(home), 0.85, r);
            let mut s = Spec::new((20, 20), Place::Anywhere).cohort(cohort);
            s.home_phone = Some(false);
            let id = w.spawn(s, r);
            let sur = w.p(id).sur;
            spawn_parents(w, hh, 20, sur, r);
            w.pm(id).hometown = Some(hh);
            id
        };
        set_high_school(w, id, &hs);
        hsmates.push(id);
    }
    link_group(w, &hsmates, r, 3);
    let entries = hsmates
        .iter()
        .map(|&id| entry(w, r, me, id, "高校の同級生", &[(Name, 0, 1.0), (Alias, 0, 1.0), (Handle, 1, 0.5), (School, 3, 1.0), (BirthMd, 0, 0.3)]))
        .collect();
    sources.push(Source {
        name: "高校の同級生 LINEグループ".into(),
        icon: "💬",
        desc: "広島の高校の同級生。表示名と本名、たまにSnapgram。".into(),
        entries,
    });

    let mut chat: Vec<(u32, &str)> = circle.iter().take(10).map(|&id| (id, "サークルの仲間")).collect();
    for _ in 0..15 {
        let id = w.spawn(Spec::new((17, 28), Place::Anywhere), r);
        w.add_family(id, r);
        chat.push((id, "ネットの友達"));
    }
    let entries = chat.iter().map(|&(id, rel)| entry(w, r, me, id, rel, &[(Handle, 0, 1.0), (Alias, 0, 1.0)])).collect();
    sources.push(Source {
        name: "Chatter 相互フォロー".into(),
        icon: "🐦",
        desc: "相互フォローしているアカウント。".into(),
        entries,
    });

    let v = spawn_idol(w, r, 4, 0, Gender::Female, (22, 28));
    let entries = vec![entry(w, r, me, v, "推しの配信者", &[(Alias, 0, 1.0), (Handle, 0, 1.0), (BirthMd, 0, 1.0)])];
    sources.push(Source {
        name: "推しの配信者".into(),
        icon: "🎮",
        desc: "配信者の活動名と公式アカウント、公表されている誕生日。".into(),
        entries,
    });
    (me, sources)
}

fn senior(w: &mut World, r: &mut Rng) -> (u32, Vec<Source>) {
    let city = city_index("仙台市青葉区") as usize;
    let town = w.random_town(city, r);
    let mut s = Spec::new((68, 68), Place::Town(town, None)).gender(Gender::Male).kind(Kind::Persona);
    s.home_phone = Some(true);
    s.occupation = Some("年金生活");
    let me = w.spawn(s, r);
    let hh = w.p(me).household;
    let sur = w.p(me).sur;
    let mut s = Spec::new((66, 66), Place::Household(hh)).gender(Gender::Female).kind(Kind::Family);
    s.sur = Some(sur);
    s.role = Role::Spouse;
    let wife = w.spawn(s, r);
    w.link(me, wife);

    let mut s = Spec::new((40, 40), Place::City(city_index("練馬区") as usize)).gender(Gender::Male).kind(Kind::Family);
    s.sur = Some(sur);
    s.occupation = Some("会社員");
    let son = w.spawn(s, r);
    w.add_family(son, r);
    w.pm(son).hometown = Some(hh);
    let sh = w.p(son).household;
    let mut s = Spec::new((15, 15), Place::Household(sh)).kind(Kind::Family);
    s.sur = Some(sur);
    s.role = Role::Child;
    let grandchild = w.spawn(s, r);
    w.pm(grandchild).overshare = 0.85;
    w.pm(grandchild).caution = 0.2;
    let daughter = w.spawn(Spec::new((37, 37), Place::City(city_index("横浜市青葉区") as usize)).gender(Gender::Female).kind(Kind::Family), r);
    w.add_family(daughter, r);
    w.pm(daughter).hometown = Some(hh);
    for x in [son, daughter, grandchild] {
        w.link(me, x);
    }
    let tname = w.towns[town].name.clone();
    let mut sources = Vec::new();

    let mut chonai = Vec::new();
    for _ in 0..25 {
        let mut s = Spec::new((35, 85), Place::Town(town, None));
        s.home_phone = Some(true);
        let id = w.spawn(s, r);
        w.add_family(id, r);
        chonai.push(id);
    }
    link_group(w, &chonai, r, 3);
    let entries = chonai
        .iter()
        .map(|&id| entry(w, r, me, id, "町内会の会員", &[(Name, 0, 1.0), (Address, 0, 1.0), (HomePhone, 0, 1.0), (Family, 0, 0.5)]))
        .collect();
    sources.push(Source {
        name: format!("{}町内会 会員名簿", tname),
        icon: "🏘",
        desc: "町内会で配られた名簿。世帯主の氏名・住所・電話番号、一部は家族構成まで。".into(),
        entries,
    });

    let mut nenga = Vec::new();
    for _ in 0..35 {
        let place = match r.weighted(&[0.5, 0.25, 0.25]) {
            0 => Place::Pref(3),
            1 => metro(r),
            _ => Place::Anywhere,
        };
        let id = w.spawn(Spec::new((55, 80), place), r);
        w.add_family(id, r);
        nenga.push(id);
    }
    let entries = nenga
        .iter()
        .map(|&id| entry(w, r, me, id, "年賀状の相手", &[(Name, 0, 1.0), (Address, 0, 1.0), (Postal, 0, 1.0), (Family, 0, 0.4)]))
        .collect();
    sources.push(Source {
        name: "今年届いた年賀状".into(),
        icon: "🎍",
        desc: "旧友や親戚からの年賀状。住所と連名の家族。".into(),
        entries,
    });

    let mut ob = Vec::new();
    for _ in 0..20 {
        let mut s = Spec::new((60, 80), Place::Pref(3));
        s.home_phone = Some(true);
        s.occupation = Some("年金生活");
        let id = w.spawn(s, r);
        w.add_family(id, r);
        ob.push(id);
    }
    link_group(w, &ob, r, 3);
    let entries = ob
        .iter()
        .map(|&id| {
            let mut e = entry(w, r, me, id, "元同僚", &[(Name, 0, 1.0), (HomePhone, 0, 1.0), (Address, 0, 0.7)]);
            e.known.push(Known::Custom(Workplace, "元 東北セイワ電工（OB会）".into()));
            e
        })
        .collect();
    sources.push(Source {
        name: "元職場のOB会名簿".into(),
        icon: "📒",
        desc: "退職者の会の名簿。自宅電話と住所。".into(),
        entries,
    });

    let entries = vec![
        entry(w, r, me, son, "長男", &[(Name, 0, 1.0), (Address, 0, 1.0), (Mobile, 0, 1.0), (BirthDate, 0, 1.0), (Handle, 1, 0.4)]),
        entry(w, r, me, daughter, "長女", &[(Name, 0, 1.0), (Address, 0, 1.0), (Mobile, 0, 1.0), (BirthDate, 0, 1.0), (Handle, 1, 0.9)]),
        entry(w, r, me, grandchild, "孫", &[(Name, 0, 1.0), (BirthDate, 0, 1.0), (Handle, 1, 1.0)]),
    ];
    sources.push(Source {
        name: "家族と孫のSnapgram".into(),
        icon: "👪",
        desc: "子どもたちの連絡先と、孫に教えてもらったSnapgramのアカウント。".into(),
        entries,
    });
    (me, sources)
}
