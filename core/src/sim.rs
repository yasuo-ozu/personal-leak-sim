//! シミュレーションの状態・時間の進行・情報の登録。

use std::collections::{HashMap, HashSet};

use crate::data::*;
use crate::date;
use crate::facts;
use crate::incident::{self, fmt_count, fmt_pct};
use crate::model::*;
use crate::persona::{self, Known};
use crate::seed::{hash01, rng, RngHelpers};
use crate::sns::Feed;
use crate::world::World;

/// シミュレーション開始日の何日前からインシデントを生成しておくか
pub const PRE_DAYS: i32 = 730;

/// 画面に渡す不透明なトークンが指す対象
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ref {
    Post { pid: u32, pf: u8, idx: u32, via: u32 },
    Account { pid: u32, pf: u8, via: u32, how: u8 },
    Hit { inc: u32, pid: u32, via: u32 },
}

pub struct SourceRec {
    pub name: String,
    pub icon: &'static str,
    pub desc: String,
    pub entries: Vec<(u32, String, Vec<Field>)>,
}

pub struct Sim {
    pub w: World,
    /// 1970-01-01 0:00 からの経過分
    pub minute: i64,
    pub intensity: f64,
    pub persona: u32,
    pub persona_key: String,
    pub incidents: Vec<Incident>,
    pub events: Vec<Event>,
    pub targets: Vec<Target>,
    pub sources: Vec<SourceRec>,
    pub feeds: HashMap<(u32, u8), Feed>,
    refs: Vec<Ref>,
    ref_ids: HashMap<Ref, u32>,
    fact_loc: HashMap<u32, (usize, usize)>,
    clue_loc: HashMap<u32, (usize, usize)>,
    next_fact: u32,
    next_clue: u32,
    pub reveal: bool,
    /// 対人の手口に失敗したときに上がる警戒度
    pub alert: HashMap<u32, u32>,
    attempts: HashMap<(u32, usize), u32>,
    pub fake_follows: HashSet<(u32, u8)>,
    /// 記録済みの投稿・紐付け済みの流出レコード
    pub done: HashSet<u32>,
    pending: Vec<usize>,
}

fn cause_tag(cause: usize) -> &'static str {
    ["不正アクセス", "ランサムウェア", "内部不正", "内部不正", "設定ミス", "誤送信", "サプライチェーン攻撃", "フィッシング", "スクレイピング"]
        .get(cause)
        .copied()
        .unwrap_or("流出")
}

pub fn fmt_dur(m: i64) -> String {
    if m <= 0 {
        return "0分".to_string();
    }
    let (d, h, mi) = (m / 1440, (m % 1440) / 60, m % 60);
    let mut s = String::new();
    if d > 0 {
        s += &format!("{}日", d);
    }
    if h > 0 {
        s += &format!("{}時間", h);
    }
    if mi > 0 && d == 0 {
        s += &format!("{}分", mi);
    }
    s
}

impl Sim {
    pub fn new(key: &str, seed: u64, intensity: f64) -> Option<Sim> {
        let start = date::days_from_civil(2027, 4, 1);
        let mut w = World::new(seed, start);
        let mut r = rng(&[seed, 0xB01D]);
        let (persona, sources) = persona::build(key, &mut w, &mut r)?;
        let mut sim = Sim {
            w,
            minute: start as i64 * 1440 + 9 * 60,
            intensity,
            persona,
            persona_key: key.to_string(),
            incidents: Vec::new(),
            events: Vec::new(),
            targets: Vec::new(),
            sources: Vec::new(),
            feeds: HashMap::new(),
            refs: Vec::new(),
            ref_ids: HashMap::new(),
            fact_loc: HashMap::new(),
            clue_loc: HashMap::new(),
            next_fact: 1,
            next_clue: 1,
            reveal: false,
            alert: HashMap::new(),
            attempts: HashMap::new(),
            fake_follows: HashSet::new(),
            done: HashSet::new(),
            pending: Vec::new(),
        };
        for s in sources {
            sim.add_source(s);
        }
        for d in (start - PRE_DAYS)..=start {
            sim.process_day(d);
        }
        Some(sim)
    }

    pub fn start(&self) -> i32 {
        self.w.start
    }

    pub fn day(&self) -> i32 {
        self.minute.div_euclid(1440) as i32
    }

    fn add_source(&mut self, s: persona::Source) {
        let now = self.day();
        let mut entries = Vec::new();
        for e in s.entries {
            let t = match self.targets.iter().position(|t| t.person == e.person) {
                Some(t) => t,
                None => self.new_target(e.person, String::new(), &e.relation),
            };
            let mut fields = Vec::new();
            for k in e.known {
                let init = match k {
                    Known::Auto(f, v) => facts::value(&self.w, e.person, f, v, now),
                    Known::Custom(f, v) => Some(FactInit::new(f, v, e.person)),
                };
                if let Some(init) = init {
                    fields.push(init.field);
                    self.add_fact(t, init, &s.name, &s.name, Vec::new(), 0, String::new());
                }
            }
            self.refresh_label(t);
            entries.push((t as u32, e.relation, fields));
        }
        self.sources.push(SourceRec { name: s.name, icon: s.icon, desc: s.desc, entries });
    }

    pub fn new_target(&mut self, person: u32, label: String, relation: &str) -> usize {
        let id = self.targets.len() as u32;
        self.targets.push(Target {
            id,
            person,
            label,
            relation: relation.to_string(),
            facts: Vec::new(),
            clues: Vec::new(),
            spent: 0,
            log: Vec::new(),
        });
        id as usize
    }

    pub fn refresh_label(&mut self, t: usize) {
        if !self.targets[t].label.is_empty() {
            return;
        }
        let tg = &self.targets[t];
        let pick = |f: Field| tg.facts.iter().find(|x| x.init.field == f).map(|x| x.init.value.clone());
        let label = pick(Field::Name).or_else(|| pick(Field::Alias)).or_else(|| pick(Field::Handle)).unwrap_or_else(|| "（名前不明）".to_string());
        self.targets[t].label = label;
    }

    // ---- 時間 ----

    pub fn spend(&mut self, minutes: i64) -> Vec<Event> {
        let before = self.events.len();
        let d0 = self.day();
        self.minute += minutes.max(0);
        for d in (d0 + 1)..=self.day() {
            self.process_day(d);
        }
        self.events[before..].to_vec()
    }

    fn process_day(&mut self, d: i32) {
        let mut r = rng(&[self.w.seed, 0xDA7, d as i64 as u64]);
        let n = r.poisson(self.intensity / 7.0);
        for _ in 0..n {
            let id = self.incidents.len() as u32;
            let mut inc = incident::generate(&self.w, id, d, self.persona, &mut r);
            inc.persona_hit = incident::leaked(&self.w, &inc, self.persona);
            self.announce_event(&inc);
            if inc.persona_hit {
                let rec = incident::record(&self.w, &inc, self.persona);
                let labels: Vec<&str> = rec.iter().map(|f| f.field.label()).collect();
                self.events.push(Event {
                    day: d,
                    date: date::fmt(d),
                    kind: "notify",
                    title: format!("📧 {}から「個人情報流出に関するお詫び」が届きました", inc.company),
                    body: format!("あなたの {} が流出した可能性があります。", labels.join("・")),
                    incident: Some(id),
                    level: 4,
                });
            }
            if inc.circ.is_some() {
                self.pending.push(self.incidents.len());
            }
            self.incidents.push(inc);
        }
        let due: Vec<usize> = self.pending.iter().copied().filter(|&i| self.incidents[i].circ.is_some_and(|c| c <= d)).collect();
        self.pending.retain(|i| !due.contains(i));
        for i in due {
            let inc = &self.incidents[i];
            let broker = matches!(inc.cause, 2 | 3);
            self.events.push(Event {
                day: d,
                date: date::fmt(d),
                kind: "circulate",
                title: format!("🕶 {}の流出データが闇市場で出回っているのが確認された", inc.company),
                body: format!(
                    "{}件分のデータが{}。以後「流出データで名寄せ」の検索対象になります。",
                    fmt_count(inc.count),
                    if broker { "名簿業者に転売され、闇サイトでも売買されている" } else { "匿名掲示板や闇サイトで売買されている" }
                ),
                incident: Some(inc.id),
                level: 3,
            });
        }
    }

    fn announce_event(&mut self, inc: &Incident) {
        let c = &CATEGORIES[inc.cat];
        let fields: Vec<&str> = inc.fields.iter().map(|f| f.label()).collect();
        let mut body = format!(
            "{}（{}）は、{}ごろ{}が発生し、{}を含む{}件（{}）の個人情報が流出した可能性があると発表した。",
            inc.company,
            if inc.service.is_empty() { c.label.to_string() } else { inc.service.clone() },
            date::fmt_ym(inc.occur),
            incident::cause_label(inc.cause),
            fields.join("・"),
            fmt_count(inc.count),
            fmt_pct(inc.pct)
        );
        if let Some(p) = inc.pref {
            body += &format!("対象は主に{}の住民・利用者。", PREFS[p].0);
        }
        if let Some(g) = inc.idol {
            body += &format!("対象は「{}」のファンクラブ会員。", IDOL_GROUPS[g].0);
        }
        self.events.push(Event {
            day: inc.announce,
            date: date::fmt(inc.announce),
            kind: "announce",
            title: format!("【{}】{}、{}件の個人情報が流出か", cause_tag(inc.cause), inc.company, fmt_count(inc.count)),
            body,
            incident: Some(inc.id),
            level: incident::severity(inc),
        });
    }

    pub fn circulating(&self) -> impl Iterator<Item = &Incident> {
        let d = self.day();
        self.incidents.iter().filter(move |i| i.circ.is_some_and(|c| c <= d))
    }

    // ---- 情報の登録 ----

    #[allow(clippy::too_many_arguments)]
    pub fn add_fact(&mut self, t: usize, init: FactInit, source: &str, via: &str, parents: Vec<Parent>, cost: i64, note: String) -> Option<u32> {
        if let Some(f) = self.targets[t].facts.iter_mut().find(|f| f.init.field == init.field && f.init.value == init.value) {
            if !f.sources.iter().any(|s| s == source) {
                f.sources.push(source.to_string());
            }
            return None;
        }
        let total = cost + parents.iter().map(|p| self.total_of(*p)).max().unwrap_or(0);
        let id = self.next_fact;
        self.next_fact += 1;
        let day = self.day();
        let tg = &mut self.targets[t];
        tg.facts.push(Fact {
            id,
            init,
            sources: vec![source.to_string()],
            day,
            parents,
            via: via.to_string(),
            note,
            cost,
            total,
        });
        self.fact_loc.insert(id, (t, tg.facts.len() - 1));
        Some(id)
    }

    pub fn add_clue(&mut self, t: usize, mut c: Clue) -> Option<u32> {
        if self.targets[t].clues.iter().any(|x| x.subject == c.subject && x.day == c.day && x.text == c.text) {
            return None;
        }
        c.total = c.cost + c.parent.map(|p| self.total_of(Parent::Fact(p))).unwrap_or(0);
        c.id = self.next_clue;
        self.next_clue += 1;
        let tg = &mut self.targets[t];
        tg.clues.push(c);
        self.clue_loc.insert(self.next_clue - 1, (t, tg.clues.len() - 1));
        Some(self.next_clue - 1)
    }

    fn total_of(&self, p: Parent) -> i64 {
        match p {
            Parent::Fact(id) => self.fact(id).map(|f| f.total).unwrap_or(0),
            Parent::Clue(id) => self.clue(id).map(|c| c.total).unwrap_or(0),
        }
    }

    pub fn fact(&self, id: u32) -> Option<&Fact> {
        let &(t, i) = self.fact_loc.get(&id)?;
        self.targets.get(t)?.facts.get(i)
    }

    pub fn clue(&self, id: u32) -> Option<&Clue> {
        let &(t, i) = self.clue_loc.get(&id)?;
        self.targets.get(t)?.clues.get(i)
    }

    pub fn clue_mut(&mut self, id: u32) -> Option<&mut Clue> {
        let &(t, i) = self.clue_loc.get(&id)?;
        self.targets.get_mut(t)?.clues.get_mut(i)
    }

    pub fn fact_of(&self, t: usize, id: u32) -> Result<&Fact, String> {
        match self.fact_loc.get(&id) {
            Some(&(tt, i)) if tt == t => Ok(&self.targets[t].facts[i]),
            _ => Err("指定された情報が見つかりません".to_string()),
        }
    }

    pub fn clue_of(&self, t: usize, id: u32) -> Result<&Clue, String> {
        match self.clue_loc.get(&id) {
            Some(&(tt, i)) if tt == t => Ok(&self.targets[t].clues[i]),
            _ => Err("指定された写真が見つかりません".to_string()),
        }
    }

    pub fn token(&mut self, r: Ref) -> u32 {
        if let Some(&t) = self.ref_ids.get(&r) {
            return t;
        }
        let t = self.refs.len() as u32;
        self.refs.push(r);
        self.ref_ids.insert(r, t);
        t
    }

    pub fn resolve(&self, token: u32) -> Result<Ref, String> {
        self.refs.get(token as usize).copied().ok_or_else(|| "無効な操作です".to_string())
    }

    pub fn accessible(&self, pid: u32, pf: u8) -> bool {
        self.w.p(pid).account(pf).is_some_and(|a| a.public || a.persona_follows) || self.fake_follows.contains(&(pid, pf))
    }

    pub fn feed(&mut self, pid: u32, pf: u8) -> &Feed {
        let day = self.day();
        let w = &self.w;
        let f = self.feeds.entry((pid, pf)).or_insert_with(|| Feed::new(w, pid, pf));
        f.extend(w, pid, pf, day);
        f
    }

    /// 対人の手口の成否。失敗すると相手の警戒度が上がり、以後成功しにくくなる
    pub fn se_roll(&mut self, pid: u32, action: usize, base: f64) -> bool {
        let alert = self.alert.get(&pid).copied().unwrap_or(0);
        if alert >= 3 {
            return false;
        }
        let n = {
            let e = self.attempts.entry((pid, action)).or_insert(0);
            *e += 1;
            *e
        };
        let p = (base * 0.5f64.powi(alert as i32)).clamp(0.0, 0.95);
        let ok = hash01(&[self.w.seed, 0x5E, pid as u64, action as u64, n as u64]) < p;
        if !ok {
            *self.alert.entry(pid).or_insert(0) += 1;
        }
        ok
    }

    pub fn attempt_no(&self, pid: u32, action: usize) -> u32 {
        self.attempts.get(&(pid, action)).copied().unwrap_or(0)
    }

    pub fn raise_alert(&mut self, pid: u32, n: u32) {
        *self.alert.entry(pid).or_insert(0) += n;
    }
}
