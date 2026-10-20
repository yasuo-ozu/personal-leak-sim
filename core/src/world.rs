//! 架空の人物・世帯・町を生成して保持する。

use std::collections::{HashMap, HashSet};

use rand::distr::{Alphanumeric, SampleString};
use rand::RngExt;

use crate::data::*;
use crate::date;
use crate::model::*;
use crate::seed::{mix, rng, Rng, RngHelpers};

pub enum Place {
    Household(usize),
    Town(usize, Option<u8>),
    City(usize),
    Pref(usize),
    Anywhere,
}

pub struct Spec {
    pub gender: Option<Gender>,
    pub age: (i32, i32),
    /// 学年の起点年（指定すると同学年になる）
    pub cohort: Option<i32>,
    pub sur: Option<usize>,
    pub given: Option<Given>,
    pub place: Place,
    pub kind: Kind,
    pub role: Role,
    pub school: Option<String>,
    pub workplace: Option<(String, String)>,
    pub univ: Option<usize>,
    pub occupation: Option<&'static str>,
    pub home_phone: Option<bool>,
    pub idol: Option<(usize, usize)>,
}

impl Spec {
    pub fn new(age: (i32, i32), place: Place) -> Spec {
        Spec {
            gender: None,
            age,
            cohort: None,
            sur: None,
            given: None,
            place,
            kind: Kind::Circle,
            role: Role::Head,
            school: None,
            workplace: None,
            univ: None,
            occupation: None,
            home_phone: None,
            idol: None,
        }
    }
    pub fn gender(mut self, g: Gender) -> Self {
        self.gender = Some(g);
        self
    }
    pub fn cohort(mut self, y: i32) -> Self {
        self.cohort = Some(y);
        self
    }
    pub fn school(mut self, s: &str) -> Self {
        self.school = Some(s.to_string());
        self
    }
    pub fn kind(mut self, k: Kind) -> Self {
        self.kind = k;
        self
    }
}

const FACULTIES: &[&str] = &["工学部", "文学部", "経済学部", "法学部", "理学部", "教育学部", "商学部"];

pub struct World {
    pub seed: u64,
    pub start: i32,
    pub persons: Vec<Person>,
    pub households: Vec<Household>,
    pub towns: Vec<Town>,
    pub stations: Vec<Station>,
    city_towns: HashMap<usize, Vec<usize>>,
    high_schools: HashMap<usize, Vec<String>>,
    cache: HashMap<u64, Vec<u32>>,
}

pub fn find_given(kanji: &str) -> Option<(Gender, usize, Given)> {
    for era in (0..6).rev() {
        if let Some(g) = GIVEN_M[era].iter().find(|g| g.0 == kanji) {
            return Some((Gender::Male, era, *g));
        }
        if let Some(g) = GIVEN_F[era].iter().find(|g| g.0 == kanji) {
            return Some((Gender::Female, era, *g));
        }
    }
    None
}

pub fn mobile_number(r: &mut Rng) -> String {
    // 0X0-0XXX-XXXX は割り当てのない番号帯（架空の番号）
    format!("{}-0{}-{}", r.pick(&["090", "080", "070"]), r.digits(3), r.digits(4))
}

pub fn home_number(pref: usize, r: &mut Rng) -> String {
    // 市内局番を 0 始まりにして実在しない番号にする
    let a = PREFS[pref].4;
    match a.len() {
        2 => format!("{}-0{}-{}", a, r.digits(3), r.digits(4)),
        3 => format!("{}-0{}-{}", a, r.digits(2), r.digits(4)),
        _ => format!("{}-0{}-{}", a, r.digits(1), r.digits(4)),
    }
}

pub fn random_company(r: &mut Rng) -> (String, String) {
    let (p, pr) = *r.pick(COMPANY_P);
    let (s, sr) = *r.pick(COMPANY_S);
    (format!("株式会社{}{}", p, s), format!("{}-{}.example", pr, sr))
}

impl World {
    pub fn new(seed: u64, start: i32) -> Self {
        World {
            seed,
            start,
            persons: Vec::new(),
            households: Vec::new(),
            towns: Vec::new(),
            stations: Vec::new(),
            city_towns: HashMap::new(),
            high_schools: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    pub fn p(&self, id: u32) -> &Person {
        &self.persons[id as usize]
    }

    pub fn pm(&mut self, id: u32) -> &mut Person {
        &mut self.persons[id as usize]
    }

    pub fn addr(&self, id: u32) -> &Address {
        &self.households[self.p(id).household].addr
    }

    pub fn station_of(&self, id: u32) -> &Station {
        &self.stations[self.towns[self.addr(id).town].station]
    }

    pub fn town_of(&self, id: u32) -> &Town {
        &self.towns[self.addr(id).town]
    }

    pub fn towns_of(&mut self, city: usize) -> Vec<usize> {
        if let Some(v) = self.city_towns.get(&city) {
            return v.clone();
        }
        let mut r = rng(&[self.seed, 0x7041, city as u64]);
        let (pref, _, clat, clon, _, spread) = CITIES[city];
        let klat = 1.0 / 111.0;
        let klon = 1.0 / (111.0 * clat.to_radians().cos());
        let mut used = HashSet::new();
        let mut sts = Vec::new();
        for _ in 0..r.random_range(3..=4) {
            let name = loop {
                let n = format!("{}{}", r.pick(TOWN_A), r.pick(&["", "台", "ヶ丘", "町", "公園", "中央"]));
                if used.insert(n.clone()) {
                    break n;
                }
            };
            sts.push(self.stations.len());
            self.stations.push(Station {
                city,
                name: format!("{}駅", name),
                lat: clat + r.normal() * spread * 0.45 * klat,
                lon: clon + r.normal() * spread * 0.45 * klon,
            });
        }
        let mut tused = HashSet::new();
        let mut ids = Vec::new();
        for _ in 0..10 {
            let name = loop {
                let n = format!("{}{}", r.pick(TOWN_A), r.pick(TOWN_B));
                if tused.insert(n.clone()) {
                    break n;
                }
            };
            let lat = clat + r.normal() * spread * 0.5 * klat;
            let lon = clon + r.normal() * spread * 0.5 * klon;
            let station = *sts
                .iter()
                .min_by(|&&a, &&b| {
                    let da = (self.stations[a].lat - lat).powi(2) + (self.stations[a].lon - lon).powi(2);
                    let db = (self.stations[b].lat - lat).powi(2) + (self.stations[b].lon - lon).powi(2);
                    da.partial_cmp(&db).unwrap()
                })
                .unwrap();
            ids.push(self.towns.len());
            self.towns.push(Town {
                city,
                name,
                chomes: r.random_range(2..=6),
                lat,
                lon,
                postal: format!("{}{}-{}", PREFS[pref].5, r.digits(1), r.digits(4)),
                station,
            });
        }
        self.city_towns.insert(city, ids.clone());
        ids
    }

    pub fn random_town(&mut self, city: usize, r: &mut Rng) -> usize {
        let t = self.towns_of(city);
        *r.pick(&t)
    }

    pub fn random_city(pref: usize, r: &mut Rng) -> usize {
        let idx: Vec<usize> = (0..CITIES.len()).filter(|&i| CITIES[i].0 == pref).collect();
        let w: Vec<f64> = idx.iter().map(|&i| CITIES[i].4).collect();
        idx[r.weighted(&w)]
    }

    pub fn random_pref(r: &mut Rng) -> usize {
        let w: Vec<f64> = PREFS.iter().map(|p| p.3).collect();
        r.weighted(&w)
    }

    pub fn make_address(&mut self, town: usize, chome: Option<u8>, r: &mut Rng) -> Address {
        let t = &self.towns[town];
        let chome = chome.unwrap_or_else(|| r.random_range(1..=t.chomes)).min(t.chomes.max(1));
        let ang = chome as f64 * 1.3;
        let building = if r.chance(0.35) {
            Some(format!(
                "{}{} {}0{}",
                r.pick(BUILDINGS),
                r.pick(TOWN_A),
                r.random_range(1..=8),
                r.random_range(1..=8)
            ))
        } else {
            None
        };
        Address {
            pref: CITIES[t.city].0,
            city: t.city,
            town,
            chome,
            banchi: r.random_range(1..=30),
            go: r.random_range(1..=25),
            building,
            lat: t.lat + ang.sin() * 0.0025 + r.normal() * 0.0008,
            lon: t.lon + ang.cos() * 0.003 + r.normal() * 0.001,
        }
    }

    pub fn address_text(&self, a: &Address) -> String {
        format!(
            "{}{}{}{}丁目{}-{}{}",
            PREFS[a.pref].0,
            CITIES[a.city].1,
            self.towns[a.town].name,
            a.chome,
            a.banchi,
            a.go,
            a.building.as_ref().map(|b| format!(" {}", b)).unwrap_or_default()
        )
    }

    pub fn new_household(&mut self, place: &Place, phone_prob: f64, r: &mut Rng) -> usize {
        let (town, chome) = match *place {
            Place::Town(t, c) => (t, c),
            Place::City(c) => (self.random_town(c, r), None),
            Place::Pref(p) => {
                let c = Self::random_city(p, r);
                (self.random_town(c, r), None)
            }
            Place::Anywhere | Place::Household(_) => {
                let p = Self::random_pref(r);
                let c = Self::random_city(p, r);
                (self.random_town(c, r), None)
            }
        };
        let addr = self.make_address(town, chome, r);
        let phone = if r.chance(phone_prob) { Some(home_number(addr.pref, r)) } else { None };
        self.households.push(Household { addr, phone, members: Vec::new() });
        self.households.len() - 1
    }

    pub fn high_school(&mut self, pref: usize, r: &mut Rng) -> String {
        if !self.high_schools.contains_key(&pref) {
            let mut hr = rng(&[self.seed, 0x4853, pref as u64]);
            let pre = match pref {
                0 => "道立",
                12 => "都立",
                25 | 26 => "府立",
                _ => "県立",
            };
            let mut names: Vec<&str> = HS_NAMES.to_vec();
            rand::seq::SliceRandom::shuffle(names.as_mut_slice(), &mut hr);
            let mut v: Vec<String> = names[..5].iter().map(|n| format!("{}{}高校", pre, n)).collect();
            v.push(format!("私立{}学園高校", names[5]));
            self.high_schools.insert(pref, v);
        }
        r.pick(&self.high_schools[&pref]).clone()
    }

    pub fn spawn(&mut self, spec: Spec, r: &mut Rng) -> u32 {
        let id = self.persons.len() as u32;
        let gender = spec.gender.unwrap_or(if r.random_bool(0.5) { Gender::Male } else { Gender::Female });
        let birth = if let Some(y) = spec.cohort {
            date::days_from_civil(y, 4, 2) + r.random_range(0..365)
        } else {
            let age = r.random_range(spec.age.0..=spec.age.1.max(spec.age.0));
            self.start - age * 365 - age / 4 - r.random_range(1..365)
        };
        let by = date::year(birth);
        let (bm, bd) = date::md(birth);
        let era = era_of_year(by);
        let given = spec.given.unwrap_or_else(|| *r.pick(if gender == Gender::Male { GIVEN_M[era] } else { GIVEN_F[era] }));
        let sur = spec.sur.unwrap_or_else(|| {
            let w: Vec<f64> = SURNAMES.iter().map(|s| s.3).collect();
            r.weighted(&w)
        });
        let age = date::age_at(birth, self.start);
        let household = match spec.place {
            Place::Household(h) => h,
            ref place => {
                let prob = match spec.home_phone {
                    Some(true) => 1.0,
                    Some(false) => 0.0,
                    None => match age {
                        ..=22 => 0.6,
                        23..=34 => 0.15,
                        35..=59 => 0.5,
                        _ => 0.85,
                    },
                };
                self.new_household(place, prob, r)
            }
        };
        let (pref, city, town) = {
            let a = &self.households[household].addr;
            (a.pref, a.city, a.town)
        };
        let tname = self.towns[town].name.clone();
        let station = self.stations[self.towns[town].station].name.clone();
        let muni = if pref == 12 && CITIES[city].1.ends_with('区') { "区立" } else { "市立" };
        let elem = format!("{}{}小学校", muni, tname);
        let middle = format!("{}{}中学校", muni, tname);
        let hs = self.high_school(pref, r);

        let mut past: Vec<String> = Vec::new();
        let mut univ_idx = None;
        let (occ_default, mut school): (&'static str, Option<String>) = {
            match age {
                ..=5 => ("未就学児", None),
                6..=11 => ("小学生", Some(spec.school.clone().unwrap_or(elem.clone()))),
                12..=14 => {
                    past.push(elem.clone());
                    ("中学生", Some(spec.school.clone().unwrap_or(middle.clone())))
                }
                15..=17 => {
                    past.extend([elem.clone(), middle.clone()]);
                    ("高校生", Some(spec.school.clone().unwrap_or(hs.clone())))
                }
                18..=21 if spec.univ.is_some() || r.chance(0.55) => {
                    past.extend([elem.clone(), middle.clone(), hs.clone()]);
                    let u = spec.univ.unwrap_or_else(|| r.random_range(0..UNIVS.len()));
                    univ_idx = Some(u);
                    ("大学生", Some(spec.school.clone().unwrap_or_else(|| format!("{} {}", UNIVS[u].0, r.pick(FACULTIES)))))
                }
                _ => {
                    past.extend([elem.clone(), middle.clone(), hs.clone()]);
                    let last = if age >= 22 && r.chance(0.45) {
                        let u = r.random_range(0..UNIVS.len());
                        let s = format!("{} {}", UNIVS[u].0, r.pick(FACULTIES));
                        past.push(s.clone());
                        s
                    } else {
                        hs.clone()
                    };
                    let female = gender == Gender::Female;
                    let occ = if age >= 65 {
                        ["年金生活", "パート・アルバイト", "自営業"][r.weighted(&[0.75, 0.12, 0.13])]
                    } else if age < 22 {
                        ["会社員", "パート・アルバイト"][r.weighted(&[0.6, 0.4])]
                    } else {
                        let w = [
                            0.62,
                            0.07,
                            0.07,
                            if female { 0.2 } else { 0.08 },
                            if female { 0.15 } else { 0.01 },
                            0.02,
                        ];
                        ["会社員", "公務員", "自営業", "パート・アルバイト", if female { "専業主婦" } else { "専業主夫" }, "無職"]
                            [r.weighted(&w)]
                    };
                    (occ, Some(last))
                }
            }
        };
        let occupation = spec.occupation.unwrap_or(occ_default);
        if spec.occupation.is_some() && spec.school.is_some() {
            school = spec.school.clone();
        }
        if occupation == "大学生" && univ_idx.is_none() {
            univ_idx = Some(spec.univ.unwrap_or_else(|| r.random_range(0..UNIVS.len())));
        }

        let mut dept = None;
        let mut work_email = None;
        let workplace = match occupation {
            "会社員" => {
                let (name, dom) = spec.workplace.clone().unwrap_or_else(|| random_company(r));
                work_email = Some(format!("{}.{}@{}", given.2, SURNAMES[sur].2, dom));
                dept = Some(*r.pick(DEPTS));
                Some(name)
            }
            "公務員" => {
                work_email = Some(format!("{}-{}@city{}.example", SURNAMES[sur].2, given.2, city));
                dept = Some(*r.pick(&["市民課", "税務課", "福祉課", "総務課"]));
                Some(format!("{}役所", CITIES[city].1))
            }
            "自営業" => Some(format!(
                "{}{}",
                SURNAMES[sur].0,
                r.pick(&["工務店", "商店", "鍼灸院", "デザイン事務所", "カフェ", "理容室"])
            )),
            "パート・アルバイト" => Some(format!(
                "{}{} {}店",
                r.pick(COMPANY_NAME_P),
                r.pick(&["マート", "ストア", "ドラッグ"]),
                tname
            )),
            _ => spec.workplace.clone().map(|w| w.0),
        };
        let income = match occupation {
            "会社員" | "公務員" => Some(((300 + (age - 22).max(0) * 10) as f64 * r.rangef(0.75, 1.4)).clamp(250.0, 1500.0) as u32),
            "自営業" => Some(r.random_range(200..=900)),
            "パート・アルバイト" => Some(r.random_range(60..=160)),
            "年金生活" => Some(r.random_range(150..=300)),
            _ => None,
        };
        let school_email = univ_idx.map(|u| format!("s{}{}@{}.example", (by + 18) % 100, r.digits(5), UNIVS[u].1));

        let mmdd = format!("{:02}{:02}", bm, bd);
        let yy = by.rem_euclid(100);
        let gr = given.2;
        let sr = SURNAMES[sur].2;
        let pet = if r.chance(0.3) { Some((r.random_range(0..PET_KINDS.len()), r.random_range(0..PET_NAMES.len()))) } else { None };
        let idol = spec.idol.or_else(|| {
            if (12..=35).contains(&age) && r.chance(0.22) {
                let g = r.weighted(&[1.2, 1.0, 1.0, 0.6, 0.5]);
                Some((g, r.random_range(0..IDOL_GROUPS[g].1.len())))
            } else {
                None
            }
        });
        let mut hobbies: Vec<usize> = Vec::new();
        for _ in 0..r.random_range(1..=3) {
            let h = r.random_range(0..HOBBIES.len());
            if !hobbies.contains(&h) {
                hobbies.push(h);
            }
        }
        let health = if r.chance(if age > 45 { 0.35 } else { 0.12 }) { Some(r.random_range(0..HEALTH.len())) } else { None };
        let base_handle = loop {
            let h = match r.random_range(0..6) {
                0 => format!("{}_{}", gr, mmdd),
                1 => format!("{}{:02}", gr, yy),
                2 => format!("{}_{}", sr, gr),
                3 => match pet {
                    Some((_, n)) => format!("{}_{}", PET_NAMES[n].1, gr),
                    None => continue,
                },
                4 => format!("{}.{}{}", gr, &sr[..1], r.random_range(10..99)),
                _ => format!("{}{}", gr, r.random_range(100..999)),
            };
            break h;
        };
        let pass_weak = r.chance(0.6);
        let password = if pass_weak {
            match r.random_range(0..6) {
                0 if pet.is_some() => format!("{}{}", PET_NAMES[pet.unwrap().1].1, by),
                1 => format!("{}{}", gr, mmdd),
                2 => base_handle.clone(),
                3 => "password1".to_string(),
                4 => format!("{}{}{}", sr, yy, bm),
                _ => format!("{}1234", gr),
            }
        } else {
            format!("{}!", Alphanumeric.sample_string(r, 11))
        };
        let gk = given.1;
        let alias = if age >= 55 {
            if r.chance(0.6) { format!("{} {}", SURNAMES[sur].0, given.0) } else { gk.to_string() }
        } else {
            match r.random_range(0..5) {
                0 => gk.to_string(),
                1 => format!("{}{}", gk, r.pick(&["🌸", "⚽", "🍙", "✨", "🐾", "🎧"])),
                2 => format!(
                    "{}{}",
                    SURNAMES[sur].1.chars().take(2).collect::<String>(),
                    gk.chars().take(2).collect::<String>()
                ),
                3 => {
                    let mut c = gr.chars();
                    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                }
                _ => format!("{}@{}垢", gk, HOBBIES[hobbies[0]]),
            }
        };
        let overshare = match age {
            ..=19 => r.rangef(0.35, 0.95),
            20..=29 => r.rangef(0.3, 0.9),
            30..=59 => r.rangef(0.15, 0.75),
            _ => r.rangef(0.1, 0.6),
        };
        let caution = (r.rangef(0.2, 0.9)
            - if age >= 70 { 0.15 } else { 0.0 }
            - if age <= 19 { 0.1 } else { 0.0 })
        .clamp(0.05, 0.95);
        let mut secqs = vec![Field::School, Field::BirthDate, Field::Family];
        if pet.is_some() {
            secqs.push(Field::Pet);
            secqs.push(Field::Pet);
        }
        let secq = *r.pick(&secqs);
        let routine = match occupation {
            "会社員" | "公務員" => format!(
                "平日{}時{}ごろ家を出て{}から通勤、帰宅は{}時ごろ",
                r.random_range(6..=8),
                r.pick(&["", "15分", "半", "45分"]),
                station,
                r.random_range(19..=22)
            ),
            "高校生" | "中学生" => format!("平日7時半ごろ家を出て登校、部活後{}時ごろ{}経由で帰宅", r.random_range(17..=19), station),
            "小学生" => format!("平日8時前に登校、{}時ごろ一人で下校", r.random_range(14..=16)),
            "大学生" => format!(
                "{}曜は1限から。夜は{}時ごろまでアルバイト",
                r.pick(&["月", "火", "水", "木", "金"]),
                r.random_range(21..=23)
            ),
            "専業主婦" | "専業主夫" => format!("平日{}時ごろ{}近くのスーパーへ買い物", r.random_range(10..=16), station),
            "年金生活" | "無職" => format!("毎朝{}時ごろ近所を散歩、日中はほぼ在宅", r.random_range(5..=8)),
            "未就学児" => "平日は保育園に通う".to_string(),
            _ => format!("{}をよく利用、週末は{}", station, HOBBIES[hobbies[0]]),
        };
        let assets = if age >= 60 && r.chance(0.5) {
            Some(r.pick(&["自宅に現金を保管している様子", "持ち家・ローン完済、退職金を預金中", "株式・投資信託を保有"]).to_string())
        } else if (30..60).contains(&age) && r.chance(0.2) {
            Some(r.pick(&["住宅ローン返済中", "投資用マンションを所有"]).to_string())
        } else {
            None
        };
        let mobile = if r.chance(match age {
            ..=9 => 0.0,
            10..=12 => 0.5,
            13..=15 => 0.85,
            16..=74 => 0.98,
            _ => 0.75,
        }) {
            Some(mobile_number(r))
        } else {
            None
        };
        let mut emails = Vec::new();
        if age >= 12 && r.chance(0.95) {
            let domain = if age >= 60 && r.chance(0.5) { *r.pick(MOBILE_DOMAINS) } else { *r.pick(PERSONAL_DOMAINS) };
            let local = match r.random_range(0..4) {
                0 => format!("{}.{}{:02}", gr, sr, yy),
                1 => base_handle.replace('_', "."),
                2 => format!("{}{}{}", sr, &gr[..1], r.random_range(10..99)),
                _ => format!("{}-{}", gr, mmdd),
            };
            emails.push(format!("{}@{}", local, domain));
            if r.chance(0.3) {
                emails.push(format!("{}.{}{}@{}", sr, gr, r.digits(3), r.pick(PERSONAL_DOMAINS)));
            }
        }
        let card = if age >= 18 && r.chance(0.85) { Some(r.digits(4)) } else { None };
        let card_exp = format!("{:02}/{}", r.random_range(1..=12), r.random_range(28..=32));

        self.persons.push(Person {
            id,
            kind: spec.kind,
            sur,
            given,
            gender,
            birth,
            household,
            role: spec.role,
            hometown: None,
            mobile,
            emails,
            work_email,
            school_email,
            accounts: Vec::new(),
            occupation,
            school,
            past_schools: past,
            workplace,
            dept,
            income,
            card,
            card_exp,
            pass_reuse: r.chance(if pass_weak { 0.75 } else { 0.35 }),
            password,
            pass_weak,
            pet,
            idol,
            hobbies,
            health,
            alias,
            stage: None,
            base_handle,
            overshare,
            caution,
            secq,
            routine,
            assets,
            friends: Vec::new(),
            phantom_inc: None,
        });
        self.households[household].members.push(id);
        if spec.kind != Kind::Idol {
            self.create_accounts(id, r);
        }
        id
    }

    fn create_accounts(&mut self, id: u32, r: &mut Rng) {
        let age = self.p(id).age_at(self.start);
        let probs = match age {
            ..=12 => [0.0, 0.0, 0.0],
            13..=19 => [0.6, 0.8, 0.05],
            20..=29 => [0.6, 0.7, 0.2],
            30..=39 => [0.55, 0.6, 0.45],
            40..=59 => [0.35, 0.4, 0.45],
            _ => [0.1, 0.12, 0.25],
        };
        for (pf, &pr) in probs.iter().enumerate() {
            if r.chance(pr) {
                self.add_account(id, pf as u8, r);
            }
        }
    }

    pub fn add_account(&mut self, id: u32, platform: u8, r: &mut Rng) {
        if self.p(id).account(platform).is_some() {
            return;
        }
        let start = self.start;
        let (city_txt, (bm, bd)) = {
            let a = self.addr(id);
            (format!("{}{}", PREFS[a.pref].0, CITIES[a.city].1), date::md(self.p(id).birth))
        };
        let p = self.p(id);
        let age = p.age_at(start);
        let o = p.overshare;
        let handle = if platform == 2 {
            format!("{}.{}.{}", p.given.2, p.sur_romaji(), r.digits(3))
        } else if r.chance(0.65) {
            p.base_handle.clone()
        } else {
            format!("{}_{}", p.base_handle, r.digits(2))
        };
        let display = if platform == 2 { p.full_name() } else { p.stage.clone().unwrap_or_else(|| p.alias.clone()) };
        let mut bio = Vec::new();
        let mut parts: Vec<String> = Vec::new();
        if p.kind == Kind::Idol {
            let (g, _) = p.idol.unwrap_or((0, 0));
            parts.push(format!("{}の{}です✨ 公式アカウント", IDOL_GROUPS[g].0, display));
            parts.push(format!("🎂{}/{}", bm, bd));
            bio.push((Field::BirthMd, 0));
        } else if platform == 2 {
            if r.chance(0.8) {
                parts.push(format!("居住地: {}", city_txt));
                bio.push((Field::City, 0));
            }
            if let Some(s) = &p.school {
                if r.chance(0.8) {
                    parts.push(format!("学歴: {}", s));
                    bio.push((Field::School, 0));
                }
            }
            if let Some(w) = &p.workplace {
                if r.chance(0.75) {
                    parts.push(format!("勤務先: {}", w));
                    bio.push((Field::Workplace, 0));
                }
            }
            if r.chance(0.4) {
                parts.push(format!("誕生日: {}月{}日", bm, bd));
                bio.push((Field::BirthMd, 0));
            }
        } else {
            if let (true, Some(s)) = (p.is_student(), &p.school) {
                if r.chance(0.3 + 0.5 * o) {
                    parts.push(format!("🏫{}", s));
                    bio.push((Field::School, 0));
                }
            }
            if r.chance(0.2 + 0.4 * o) {
                parts.push(format!("📍{}", CITIES[self.addr(id).city].1));
                bio.push((Field::City, 0));
            }
            if r.chance(0.15 + 0.4 * o) {
                parts.push(format!("🎂{}/{}", bm, bd));
                bio.push((Field::BirthMd, 0));
            }
            if let Some((g, _)) = p.idol {
                if r.chance(0.6) {
                    parts.push(format!("{}推し", IDOL_GROUPS[g].0));
                    bio.push((Field::Idol, 0));
                }
            }
            if r.chance(0.5) {
                parts.push(format!("{}好き", HOBBIES[p.hobbies[0]]));
                bio.push((Field::Hobby, 0));
            }
            if let (false, Some(w)) = (p.is_student(), &p.workplace) {
                if r.chance(0.1 + 0.2 * o) {
                    parts.push(format!("{}勤務", w));
                    bio.push((Field::Workplace, 0));
                }
            }
        }
        let is_idol = p.kind == Kind::Idol;
        let public = is_idol
            || r.chance(match platform {
                0 => 0.8 - p.caution * 0.3,
                1 => 0.7 - p.caution * 0.4,
                _ => 0.8 - p.caution * 0.3,
            });
        let rate = if is_idol {
            5.0
        } else {
            match age {
                ..=19 => r.rangef(1.5, 5.0),
                20..=39 => r.rangef(0.8, 4.0),
                _ => r.rangef(0.3, 2.0),
            }
        };
        let acc = Account {
            platform,
            handle,
            display,
            public,
            by_phone: !is_idol && r.chance(0.6),
            by_email: !is_idol && r.chance(0.45),
            since: start - r.random_range(200..3000),
            rate,
            bio,
            bio_text: if parts.is_empty() { "（自己紹介なし）".to_string() } else { parts.join(" / ") },
            selfies: is_idol || r.chance(0.3 + 0.5 * o),
            persona_follows: false,
        };
        self.pm(id).accounts.push(acc);
    }

    pub fn link(&mut self, a: u32, b: u32) {
        if a == b {
            return;
        }
        if !self.p(a).friends.contains(&b) {
            self.pm(a).friends.push(b);
        }
        if !self.p(b).friends.contains(&a) {
            self.pm(b).friends.push(a);
        }
    }

    /// 同居家族を生成する
    pub fn add_family(&mut self, id: u32, r: &mut Rng) {
        let (age, h, sur, gender) = {
            let p = self.p(id);
            (p.age_at(self.start), p.household, p.sur, p.gender)
        };
        let mut fam = Vec::new();
        if age < 23 {
            self.pm(id).role = Role::Child;
            let fa = age + r.random_range(26..=36);
            let ma = age + r.random_range(24..=34);
            if r.chance(0.9) {
                let mut s = Spec::new((fa, fa), Place::Household(h)).gender(Gender::Male).kind(Kind::Family);
                s.sur = Some(sur);
                fam.push(self.spawn(s, r));
            }
            let mut s = Spec::new((ma, ma), Place::Household(h)).gender(Gender::Female).kind(Kind::Family);
            s.sur = Some(sur);
            s.role = Role::Spouse;
            fam.push(self.spawn(s, r));
            if r.chance(0.5) {
                let sa = ((age - 6).max(1), (age + 6).min(ma - 20).max(1));
                let mut s = Spec::new(sa, Place::Household(h)).kind(Kind::Family);
                s.sur = Some(sur);
                s.role = Role::Child;
                fam.push(self.spawn(s, r));
            }
        } else {
            let married = r.chance(match age {
                ..=34 => 0.4,
                35..=64 => 0.75,
                _ => 0.7,
            });
            if married {
                let sa = (age + r.random_range(-4..=4)).max(20);
                let g = if gender == Gender::Male { Gender::Female } else { Gender::Male };
                let mut s = Spec::new((sa, sa), Place::Household(h)).gender(g).kind(Kind::Family);
                s.sur = Some(sur);
                s.role = Role::Spouse;
                fam.push(self.spawn(s, r));
                if (27..65).contains(&age) && r.chance(0.7) {
                    for _ in 0..r.random_range(1..=3) {
                        let ca = (age - 28 + r.random_range(-4..=4)).clamp(0, (age - 22).max(0));
                        let mut s = Spec::new((ca, ca), Place::Household(h)).kind(Kind::Family);
                        s.sur = Some(sur);
                        s.role = Role::Child;
                        fam.push(self.spawn(s, r));
                    }
                }
            }
        }
        for &f in &fam {
            self.link(id, f);
        }
    }

    pub fn relation(&self, subject: u32, other: u32) -> &'static str {
        let (s, o) = (self.p(subject), self.p(other));
        let male = o.gender == Gender::Male;
        match (s.role, o.role) {
            (Role::Head, Role::Spouse) | (Role::Spouse, Role::Head) => {
                if male {
                    "夫"
                } else {
                    "妻"
                }
            }
            (Role::Head | Role::Spouse, Role::Child) => {
                if male {
                    "息子"
                } else {
                    "娘"
                }
            }
            (Role::Child, Role::Head | Role::Spouse) => {
                if male {
                    "父"
                } else {
                    "母"
                }
            }
            (Role::Child, Role::Child) => match (o.birth < s.birth, male) {
                (true, true) => "兄",
                (true, false) => "姉",
                (false, true) => "弟",
                (false, false) => "妹",
            },
            _ => "同居人",
        }
    }

    pub fn family_text(&self, id: u32, now: i32) -> Option<String> {
        let p = self.p(id);
        let parts: Vec<String> = self.households[p.household]
            .members
            .iter()
            .filter(|&&m| m != id)
            .map(|&m| {
                let o = self.p(m);
                format!("{}: {}（{}歳）", self.relation(id, m), o.full_name(), o.age_at(now))
            })
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" / "))
        }
    }

    pub fn mother_name(&self, id: u32) -> Option<String> {
        let p = self.p(id);
        let h = if p.role == Role::Child { p.household } else { p.hometown? };
        self.households[h]
            .members
            .iter()
            .map(|&m| self.p(m))
            .find(|o| o.gender == Gender::Female && o.role != Role::Child)
            .map(|o| o.full_name())
    }

    pub fn has_young_child(&self, id: u32, now: i32) -> bool {
        let p = self.p(id);
        p.role != Role::Child
            && self.households[p.household]
                .members
                .iter()
                .any(|&m| self.p(m).role == Role::Child && self.p(m).age_at(now) < 10)
    }

    /// 同姓同名の別人（キャッシュされ、同じ key なら同じ人物が返る）
    pub fn namesakes(&mut self, key: u64, sur: usize, given: Given, n: usize, platform: Option<u8>, inc: Option<u32>) -> Vec<u32> {
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        let mut r = rng(&[self.seed, key]);
        let (gender, era, _) = find_given(given.0).unwrap_or((Gender::Male, 3, given));
        let mut v = Vec::new();
        for _ in 0..n {
            let (y0, y1) = era_years(era);
            let age = (date::year(self.start) - r.random_range(y0..=y1)).max(1);
            let mut s = Spec::new((age, age), Place::Anywhere).gender(gender).kind(Kind::Phantom);
            s.sur = Some(sur);
            s.given = Some(given);
            let id = self.spawn(s, &mut r);
            self.pm(id).phantom_inc = inc;
            if let Some(pf) = platform {
                self.add_account(id, pf, &mut r);
                for a in self.pm(id).accounts.iter_mut() {
                    a.public = true;
                }
            }
            v.push(id);
        }
        self.cache.insert(key, v.clone());
        v
    }

    /// 顔が似ている別人
    pub fn lookalikes(&mut self, of: u32) -> Vec<u32> {
        let key = mix(&[0xFACE, of as u64]);
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        let mut r = rng(&[self.seed, key]);
        let (g, age) = (self.p(of).gender, self.p(of).age_at(self.start));
        let mut v = Vec::new();
        for _ in 0..r.random_range(0..=2) {
            let s = Spec::new(((age - 4).max(13), age + 4), Place::Anywhere).gender(g).kind(Kind::Phantom);
            let id = self.spawn(s, &mut r);
            self.add_account(id, 1, &mut r);
            for a in self.pm(id).accounts.iter_mut() {
                a.public = true;
                a.selfies = true;
            }
            v.push(id);
        }
        self.cache.insert(key, v.clone());
        v
    }

    /// フォロワーとして見つかる知人の知人
    pub fn extended_friends(&mut self, of: u32, platform: u8) -> Vec<u32> {
        let key = mix(&[0xF011, of as u64, platform as u64]);
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        let mut r = rng(&[self.seed, key]);
        let (age, city, pref, school, is_idol) = {
            let p = self.p(of);
            let a = self.addr(of);
            (p.age_at(self.start), a.city, a.pref, p.school.clone(), p.kind == Kind::Idol)
        };
        let mut v = Vec::new();
        for _ in 0..r.random_range(3..=6) {
            let place = if is_idol {
                Place::Anywhere
            } else {
                match r.weighted(&[0.5, 0.3, 0.2]) {
                    0 => Place::City(city),
                    1 => Place::Pref(pref),
                    _ => Place::Anywhere,
                }
            };
            let ages = if is_idol { (14, 35) } else { ((age - 3).max(13), age + 3) };
            let mut s = Spec::new(ages, place).kind(Kind::Extended);
            if !is_idol && r.chance(0.4) {
                if let Some(sc) = &school {
                    if self.p(of).is_student() {
                        s.school = Some(sc.clone());
                        s.age = (age, age);
                    }
                }
            }
            let id = self.spawn(s, &mut r);
            self.add_account(id, platform, &mut r);
            if !is_idol {
                self.link(of, id);
            }
            v.push(id);
        }
        self.cache.insert(key, v.clone());
        v
    }
}
