use serde::Serialize;

use crate::data::{Given, CITIES, PREFS, SURNAMES};
use crate::date;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Name,
    Kana,
    Alias,
    Gender,
    BirthDate,
    BirthMd,
    Age,
    Pref,
    City,
    Station,
    Postal,
    Town,
    Address,
    OldAddress,
    Routine,
    Mobile,
    HomePhone,
    Email,
    Handle,
    School,
    Workplace,
    Family,
    Hobby,
    Idol,
    Pet,
    Face,
    IdDoc,
    Income,
    Assets,
    Card,
    Password,
    History,
    Health,
}

impl Field {
    pub const ALL: &'static [Field] = &[
        Field::Name, Field::Kana, Field::Alias, Field::Gender, Field::BirthDate, Field::BirthMd, Field::Age,
        Field::Pref, Field::City, Field::Station, Field::Postal, Field::Town, Field::Address, Field::OldAddress,
        Field::Routine, Field::Mobile, Field::HomePhone, Field::Email, Field::Handle, Field::School,
        Field::Workplace, Field::Family, Field::Hobby, Field::Idol, Field::Pet, Field::Face, Field::IdDoc,
        Field::Income, Field::Assets, Field::Card, Field::Password, Field::History, Field::Health,
    ];

    pub fn label(self) -> &'static str {
        use Field::*;
        match self {
            Name => "氏名",
            Kana => "フリガナ",
            Alias => "通称・表示名",
            Gender => "性別",
            BirthDate => "生年月日",
            BirthMd => "誕生日（月日）",
            Age => "年齢・学年",
            Pref => "都道府県",
            City => "市区町村",
            Station => "最寄り駅",
            Postal => "郵便番号",
            Town => "町名",
            Address => "住所",
            OldAddress => "実家・旧住所",
            Routine => "行動パターン",
            Mobile => "携帯電話",
            HomePhone => "自宅電話",
            Email => "メールアドレス",
            Handle => "SNSアカウント",
            School => "学校",
            Workplace => "勤務先",
            Family => "家族構成",
            Hobby => "趣味",
            Idol => "推し",
            Pet => "ペット",
            Face => "顔写真",
            IdDoc => "身分証画像",
            Income => "年収",
            Assets => "資産状況",
            Card => "クレジットカード",
            Password => "パスワード",
            History => "購入・利用履歴",
            Health => "健康・病歴",
        }
    }

    pub fn group(self) -> &'static str {
        use Field::*;
        match self {
            Name | Kana | Alias | Gender | BirthDate | BirthMd | Age | Face | IdDoc => "identity",
            Pref | City | Station | Postal | Town | Address | OldAddress | Routine => "location",
            Mobile | HomePhone | Email | Handle => "contact",
            School | Workplace | Family | Hobby | Idol | Pet => "social",
            Income | Assets | Card | Password | History | Health => "sensitive",
        }
    }

    /// 危険度スコアへの寄与
    pub fn risk(self) -> u32 {
        use Field::*;
        match self {
            Address => 25,
            Town => 12,
            Postal => 10,
            Station => 8,
            City => 4,
            Pref => 2,
            OldAddress => 8,
            Routine => 10,
            Name => 6,
            Kana => 1,
            Alias => 1,
            Gender => 1,
            BirthDate => 12,
            BirthMd => 4,
            Age => 2,
            Face => 10,
            IdDoc => 15,
            Mobile => 10,
            HomePhone => 6,
            Email => 6,
            Handle => 3,
            School => 8,
            Workplace => 8,
            Family => 6,
            Hobby => 1,
            Idol => 1,
            Pet => 3,
            Income => 4,
            Assets => 8,
            Card => 10,
            Password => 15,
            History => 4,
            Health => 10,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    pub fn label(self) -> &'static str {
        match self {
            Gender::Male => "男性",
            Gender::Female => "女性",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Address {
    pub pref: usize,
    pub city: usize,
    pub town: usize,
    pub chome: u8,
    pub banchi: u16,
    pub go: u16,
    pub building: Option<String>,
    pub lat: f64,
    pub lon: f64,
}

pub struct Town {
    pub city: usize,
    pub name: String,
    pub chomes: u8,
    pub lat: f64,
    pub lon: f64,
    pub postal: String,
    pub station: usize,
}

pub struct Station {
    pub city: usize,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Head,
    Spouse,
    Child,
}

pub struct Household {
    pub addr: Address,
    pub phone: Option<String>,
    pub members: Vec<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Persona,
    Circle,
    Family,
    Extended,
    Phantom,
    Idol,
}

pub struct Account {
    pub platform: u8,
    pub handle: String,
    pub display: String,
    pub public: bool,
    pub by_phone: bool,
    pub by_email: bool,
    pub since: i32,
    pub rate: f64,
    /// プロフィール欄から読み取れる情報
    pub bio: Vec<(Field, u8)>,
    pub bio_text: String,
    pub selfies: bool,
    pub persona_follows: bool,
}

pub struct Person {
    pub id: u32,
    pub kind: Kind,
    pub sur: usize,
    pub given: Given,
    pub gender: Gender,
    pub birth: i32,
    pub household: usize,
    pub role: Role,
    /// 実家（親の世帯）
    pub hometown: Option<usize>,
    pub mobile: Option<String>,
    pub emails: Vec<String>,
    pub work_email: Option<String>,
    pub school_email: Option<String>,
    pub accounts: Vec<Account>,
    pub occupation: &'static str,
    pub school: Option<String>,
    pub past_schools: Vec<String>,
    pub workplace: Option<String>,
    pub dept: Option<&'static str>,
    pub income: Option<u32>,
    pub card: Option<String>,
    pub card_exp: String,
    pub password: String,
    pub pass_weak: bool,
    pub pass_reuse: bool,
    pub pet: Option<(usize, usize)>,
    pub idol: Option<(usize, usize)>,
    pub hobbies: Vec<usize>,
    pub health: Option<usize>,
    pub alias: String,
    pub stage: Option<String>,
    pub base_handle: String,
    pub overshare: f64,
    pub caution: f64,
    pub secq: Field,
    pub routine: String,
    pub assets: Option<String>,
    pub friends: Vec<u32>,
    pub phantom_inc: Option<u32>,
}

impl Person {
    pub fn sur_kanji(&self) -> &'static str {
        SURNAMES[self.sur].0
    }
    pub fn sur_romaji(&self) -> &'static str {
        SURNAMES[self.sur].2
    }
    pub fn full_name(&self) -> String {
        format!("{} {}", SURNAMES[self.sur].0, self.given.0)
    }
    pub fn kana(&self) -> String {
        format!("{} {}", SURNAMES[self.sur].1, self.given.1)
    }
    pub fn age_at(&self, day: i32) -> i32 {
        date::age_at(self.birth, day)
    }
    pub fn account(&self, platform: u8) -> Option<&Account> {
        self.accounts.iter().find(|a| a.platform == platform)
    }
    pub fn is_student(&self) -> bool {
        matches!(self.occupation, "小学生" | "中学生" | "高校生" | "大学生")
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Geo {
    pub lat: f64,
    pub lon: f64,
    /// 推定範囲の半径（m）
    pub r: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Parent {
    Fact(u32),
    Clue(u32),
}

/// 人物データから作られた「情報」の中身
#[derive(Clone, Debug)]
pub struct FactInit {
    pub field: Field,
    pub value: String,
    /// 照合用の正規化値（SNSのIDや学校名など）
    pub key: String,
    pub subject: u32,
    pub geo: Option<Geo>,
    pub pref: Option<usize>,
    pub aux: i64,
    pub aux2: i64,
}

impl FactInit {
    pub fn new(field: Field, value: String, subject: u32) -> Self {
        FactInit { field, key: value.clone(), value, subject, geo: None, pref: None, aux: 0, aux2: 0 }
    }
}

pub struct Fact {
    pub id: u32,
    pub init: FactInit,
    pub sources: Vec<String>,
    pub day: i32,
    pub parents: Vec<Parent>,
    pub via: String,
    pub note: String,
    /// この情報を得るのにかかった時間（分）
    pub cost: i64,
    /// 出どころをたどった累計時間（分）
    pub total: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feature {
    Face,
    EyeReflection,
    PolePlate,
    DeliverySlip,
    Uniform,
    IdCard,
    Landmark,
    HouseView,
    CarPlate,
}

impl Feature {
    pub fn label(self) -> &'static str {
        match self {
            Feature::Face => "顔",
            Feature::EyeReflection => "瞳の映り込み",
            Feature::PolePlate => "電柱の住所表示",
            Feature::DeliverySlip => "宅配伝票",
            Feature::Uniform => "制服・校章",
            Feature::IdCard => "社員証",
            Feature::Landmark => "背景の建物・地形",
            Feature::HouseView => "家の外観・周囲の景色",
            Feature::CarPlate => "ナンバープレート",
        }
    }
}

pub struct Clue {
    pub id: u32,
    pub subject: u32,
    pub platform: u8,
    pub text: String,
    pub caption: String,
    pub day: i32,
    pub features: Vec<Feature>,
    pub exif: bool,
    pub analyzed: bool,
    pub exif_checked: bool,
    pub parent: Option<u32>,
    pub cost: i64,
    pub total: i64,
}

pub struct Target {
    pub id: u32,
    pub person: u32,
    pub label: String,
    pub relation: String,
    pub facts: Vec<Fact>,
    pub clues: Vec<Clue>,
    /// この人物の調査に費やした時間（分）
    pub spent: i64,
    pub log: Vec<LogEntry>,
}

pub struct LogEntry {
    pub day: i32,
    pub action: String,
    pub result: String,
    pub cost: i64,
    pub ok: bool,
}

pub struct Incident {
    pub id: u32,
    pub company: String,
    pub service: String,
    pub cat: usize,
    pub cause: usize,
    pub occur: i32,
    pub announce: i32,
    pub circ: Option<i32>,
    /// 日本人の何%か
    pub pct: f64,
    pub count: u64,
    pub fields: Vec<Field>,
    pub female: f64,
    pub age: [f64; 9],
    pub norm: f64,
    pub pref: Option<usize>,
    pub idol: Option<usize>,
    pub platform: u8,
    pub plain_pw: bool,
    pub persona_hit: bool,
}

#[derive(Clone, Serialize)]
pub struct Event {
    pub day: i32,
    pub date: String,
    pub kind: &'static str,
    pub title: String,
    pub body: String,
    pub incident: Option<u32>,
    pub level: u8,
}

#[allow(dead_code)]
pub fn pref_name(i: usize) -> &'static str {
    PREFS[i].0
}

#[allow(dead_code)]
pub fn city_name(i: usize) -> &'static str {
    CITIES[i].1
}
