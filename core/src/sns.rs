//! SNS投稿の生成。投稿は人物ごとに決定的に生成され、時間の経過とともに増える。

use rand::RngExt;

use crate::data::*;
use crate::date;
use crate::model::*;
use crate::seed::{rng, Rng, RngHelpers};
use crate::world::World;

pub struct Photo {
    pub desc: String,
    pub features: Vec<Feature>,
    pub exif: bool,
}

pub struct Post {
    pub day: i32,
    pub text: String,
    pub photo: Option<Photo>,
    pub reveals: Vec<(Field, u8)>,
}

impl Post {
    pub fn has_info(&self) -> bool {
        !self.reveals.is_empty() || self.photo.as_ref().is_some_and(|p| !p.features.is_empty() || p.exif)
    }
}

pub struct Feed {
    r: Rng,
    next: i32,
    last: i32,
    pub posts: Vec<Post>,
}

fn gap(r: &mut Rng, rate: f64) -> i32 {
    let mean = 7.0 / rate.max(0.1);
    ((-(r.random::<f64>().max(1e-9)).ln()) * mean).ceil().max(1.0) as i32
}

impl Feed {
    pub fn new(w: &World, id: u32, platform: u8) -> Feed {
        let acc = w.p(id).account(platform).expect("account");
        let mut r = rng(&[w.seed, 0xFEED, id as u64, platform as u64]);
        let begin = acc.since.max(w.start - 540);
        Feed { next: begin + gap(&mut r, acc.rate), last: begin, r, posts: Vec::new() }
    }

    pub fn extend(&mut self, w: &World, id: u32, platform: u8, until: i32) {
        let p = w.p(id);
        let Some(acc) = p.account(platform) else { return };
        while self.next <= until {
            let d = self.next;
            let bd = date::next_birthday(p.birth, self.last + 1);
            if bd <= d && platform != 2 && self.r.chance(if p.kind == Kind::Idol { 1.0 } else { 0.3 + 0.5 * p.overshare }) {
                let age = p.age_at(bd);
                let text = if p.kind == Kind::Idol {
                    format!("{}歳になりました💐 たくさんのお祝いありがとうございます！", age)
                } else {
                    format!("{}歳になりました🎂 これからもよろしく！", age)
                };
                self.posts.push(Post { day: bd, text, photo: None, reveals: vec![(Field::BirthMd, 0), (Field::Age, 0)] });
            }
            let post = make_post(w, id, platform, acc.selfies, d, &mut self.r);
            self.posts.push(post);
            self.last = d;
            self.next = d + gap(&mut self.r, acc.rate);
        }
    }
}

fn noise(p: &Person, age: i32, r: &mut Rng) -> String {
    if p.kind == Kind::Idol {
        return r
            .pick(&["本日も応援ありがとうございました✨", "新曲のMV公開されました🎬", "ライブ来てくれた皆さんありがとう！", "今日は撮影でした📸", "明日の配信もお楽しみに🌙"])
            .to_string();
    }
    let g = match age {
        ..=19 => 0,
        20..=49 => 1,
        _ => 2,
    };
    r.pick(NOISE_POSTS[g]).to_string()
}

fn make_post(w: &World, id: u32, platform: u8, selfies: bool, day: i32, r: &mut Rng) -> Post {
    let p = w.p(id);
    let o = p.overshare;
    let idol = p.kind == Kind::Idol;
    let age = p.age_at(day);
    let st = w.station_of(id).name.clone();
    let town = w.town_of(id).name.clone();
    let city = CITIES[w.addr(id).city].1;
    let student = p.is_student();
    let employed = p.workplace.is_some() && !student && !idol;
    let fam: Vec<u32> = w.households[p.household].members.iter().copied().filter(|&m| m != id).collect();
    let not_idol = if idol { 0.0 } else { 1.0 };

    let weights = [
        5.0,                                                                       // 0 雑談
        0.7 * o * not_idol,                                                        // 1 駅
        0.45 * o * not_idol,                                                       // 2 町
        0.2 * o * not_idol,                                                        // 3 市区町村
        if student && p.school.is_some() { 0.8 * o } else { 0.0 },                 // 4 学校
        if employed { if platform == 2 { 1.0 * o } else { 0.35 * o } } else { 0.0 }, // 5 職場
        if p.pet.is_some() { 0.5 } else { 0.0 },                                   // 6 ペット
        if p.idol.is_some() && !idol { 0.6 } else { 0.0 },                         // 7 推し
        0.5,                                                                       // 8 趣味
        if !fam.is_empty() { 0.35 * o * not_idol } else { 0.0 },                   // 9 家族
        0.25 * o * not_idol,                                                       // 10 生活パターン
        match platform {
            1 => 4.0,
            0 => 1.0,
            _ => 1.3,
        }, // 11 写真
    ];
    let mut post = Post { day, text: String::new(), photo: None, reveals: Vec::new() };
    match r.weighted(&weights) {
        1 => {
            post.text = r
                .pick(&[
                    "{s}の駅前に新しいカフェできてた☕",
                    "{s}で人身事故…今日遅刻確定",
                    "毎朝{s}から乗る電車ほんと満員",
                    "{s}のロータリーでイルミネーション始まってた✨",
                ])
                .replace("{s}", &st);
            post.reveals.push((Field::Station, 0));
        }
        2 => {
            post.text = r
                .pick(&["近所の{t}公園の桜が満開🌸", "{t}のお祭り、今年もにぎやかだった🏮", "{t}のパン屋さん閉店しちゃうらしい…"])
                .replace("{t}", &town);
            post.reveals.push((Field::Town, 0));
        }
        3 => {
            post.text = format!("{}に住んで{}年。なんだかんだ住みやすい", city, r.random_range(2..=12));
            post.reveals.push((Field::City, 0));
        }
        4 => {
            let s = p.school.clone().unwrap_or_default();
            post.text = r
                .pick(&["{s}の文化祭おわった〜！みんなおつかれ", "明日から{s}は期末テスト…", "{s}の体育祭、うちのクラス優勝🏆"])
                .replace("{s}", &s);
            post.reveals.push((Field::School, 0));
        }
        5 => {
            let wp = p.workplace.clone().unwrap_or_default();
            post.text = r
                .pick(&["{w}に入社して{n}年。早いなぁ", "今日も{w}で残業…", "{w}の新オフィス、景色がいい"])
                .replace("{w}", &wp)
                .replace("{n}", &r.random_range(2..=10).to_string());
            post.reveals.push((Field::Workplace, 0));
        }
        6 => {
            let (k, n) = p.pet.unwrap();
            post.text = format!("うちの{}の{}、今日もかわいすぎる🐾", PET_KINDS[k], PET_NAMES[n].0);
            post.reveals.push((Field::Pet, 0));
        }
        7 => {
            let (g, m) = p.idol.unwrap();
            post.text = if r.random_bool(0.5) {
                format!("{}（{}）のライブ当たった😭✨", IDOL_GROUPS[g].1[m], IDOL_GROUPS[g].0)
            } else {
                format!("{}の新曲ずっとリピートしてる", IDOL_GROUPS[g].0)
            };
            post.reveals.push((Field::Idol, 0));
        }
        8 => {
            let h = HOBBIES[*r.pick(&p.hobbies)];
            post.text = if r.random_bool(0.5) { format!("週末は{}。最高のリフレッシュ", h) } else { format!("{}仲間募集してます！", h) };
            post.reveals.push((Field::Hobby, 0));
        }
        9 => {
            let m = *r.pick(&fam);
            let o = w.p(m);
            post.text = format!("{}の{}が今日で{}歳！おめでとう🎉", w.relation(id, m), o.given.0, o.age_at(day));
            post.reveals.push((Field::Family, 0));
        }
        10 => {
            post.text = format!("毎朝ほぼ同じ時間に{}から電車。今日も同じ車両の同じ席", st);
            post.reveals.extend([(Field::Routine, 0), (Field::Station, 0)]);
        }
        11 => post = photo_post(w, id, selfies, day, r),
        _ => post.text = noise(p, age, r),
    }
    post
}

fn photo_post(w: &World, id: u32, selfies: bool, day: i32, r: &mut Rng) -> Post {
    let p = w.p(id);
    let o = p.overshare;
    let idol = p.kind == Kind::Idol;
    let age = p.age_at(day);
    let not_idol = if idol { 0.0 } else { 1.0 };
    let weights = [
        if idol { 3.0 } else if selfies { 1.2 } else { 0.2 },                       // 0 自撮り
        2.0,                                                                        // 1 料理
        if idol { 0.5 } else { 0.15 + 0.5 * o },                                    // 2 窓からの景色
        0.45 * o * not_idol,                                                        // 3 家の前
        0.35 * o * not_idol,                                                        // 4 宅配
        if (13..=18).contains(&age) && p.is_student() { 0.7 } else { 0.0 },         // 5 制服
        if p.workplace.is_some() && !p.is_student() && !idol { 0.25 * o } else { 0.0 }, // 6 職場
        if age >= 22 { 0.2 * not_idol } else { 0.0 },                               // 7 車
        if p.pet.is_some() { 0.8 } else { 0.0 },                                    // 8 ペット
    ];
    let home_exif = r.chance(if idol { 0.02 } else { 0.03 + 0.07 * o });
    let mut features = Vec::new();
    let mut reveals = Vec::new();
    let mut exif = false;
    let (text, desc): (String, &str) = match r.weighted(&weights) {
        0 => {
            features.push(Feature::Face);
            if r.chance(if idol { 0.35 } else { 0.08 + 0.25 * o }) {
                features.push(Feature::EyeReflection);
            }
            exif = home_exif && r.random_bool(0.5);
            (
                r.pick(&["新しい髪型にしてみた", "今日のメイク💄", "久しぶりの自撮り", "おうちでまったり🏠"]).to_string(),
                "自撮り写真（顔がはっきり写っている）",
            )
        }
        2 => {
            features.extend([Feature::Landmark, Feature::HouseView]);
            exif = home_exif;
            (
                r.pick(&["ベランダからの夕焼けがきれい🌇", "部屋の窓からの景色、今日は空気が澄んでる", "雪積もってる⛄"]).to_string(),
                "自宅の窓から撮った景色",
            )
        }
        3 => {
            if r.chance(0.75) {
                features.push(Feature::PolePlate);
            }
            features.push(Feature::HouseView);
            exif = home_exif;
            (r.pick(&["新しい自転車買った🚲", "玄関に朝顔咲いた🌺", "家の前の雪かきおわり⛄"]).to_string(), "自宅の前で撮った写真")
        }
        4 => {
            features.push(Feature::DeliverySlip);
            exif = home_exif;
            (r.pick(&["届いた！ずっと欲しかったやつ📦", "通販で頼んでたのきた〜"]).to_string(), "届いた段ボール箱の写真")
        }
        5 => {
            features.extend([Feature::Uniform, Feature::Face]);
            (r.pick(&["制服で友達と📸", "入学式🌸", "文化祭のクラスT👕"]).to_string(), "制服姿の写真")
        }
        6 => {
            features.push(Feature::IdCard);
            (r.pick(&["新しい社員証もらった", "今日からこのデスクで仕事"]).to_string(), "オフィスで撮った写真")
        }
        7 => {
            features.push(Feature::CarPlate);
            if r.chance(0.3) {
                features.push(Feature::HouseView);
            }
            (r.pick(&["洗車してピカピカ🚗", "納車しました！"]).to_string(), "自宅の駐車場で撮った車の写真")
        }
        8 => {
            let (_, n) = p.pet.unwrap();
            reveals.push((Field::Pet, 0));
            exif = home_exif;
            (format!("うちの{}、お昼寝中🐾", PET_NAMES[n].0), "ペットの写真")
        }
        _ => (r.pick(&["今日のランチ🍝", "映えるパフェ🍨", "新しくできたカフェ☕"]).to_string(), "料理の写真"),
    };
    Post { day, text, photo: Some(Photo { desc: desc.to_string(), features, exif }), reveals }
}
