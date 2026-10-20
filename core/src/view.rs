//! JS 側に渡す表示用データ。

use serde::Serialize;

use crate::actions::{Slot, ACTIONS};
use crate::data::*;
use crate::date;
use crate::incident::{self, fmt_count, fmt_pct};
use crate::model::*;
use crate::persona::PERSONAS;
use crate::sim::{fmt_dur, Ref, Sim};

#[derive(Serialize)]
pub struct KV {
    pub label: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct ChainStep {
    pub action: String,
    pub label: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct FactView {
    pub id: u32,
    pub field: Field,
    pub label: &'static str,
    pub group: &'static str,
    pub value: String,
    pub sources: Vec<String>,
    pub date: String,
    pub via: String,
    pub note: String,
    pub cost_text: String,
    pub total: i64,
    pub total_text: String,
    pub chain: Vec<ChainStep>,
    pub correct: Option<bool>,
}

#[derive(Serialize)]
pub struct ClueView {
    pub id: u32,
    pub platform: &'static str,
    pub text: String,
    pub caption: String,
    pub date: String,
    pub analyzed: bool,
    pub exif_checked: bool,
    pub features: Vec<&'static str>,
    pub street_ok: bool,
    pub chain: Vec<ChainStep>,
    pub total_text: String,
    pub correct: Option<bool>,
}

#[derive(Serialize)]
pub struct AccountView {
    pub token: u32,
    pub platform: &'static str,
    pub icon: &'static str,
    pub handle: String,
    pub display: String,
    pub bio: String,
    pub public: bool,
    pub accessible: bool,
    pub note: String,
    pub known: Option<u32>,
}

#[derive(Serialize)]
pub struct PostView {
    pub token: u32,
    pub date: String,
    pub text: String,
    pub photo: Option<String>,
    pub info: bool,
    pub done: bool,
}

#[derive(Serialize)]
pub struct FeedView {
    pub profile_token: u32,
    pub profile_info: bool,
    pub profile_done: bool,
    pub account: AccountView,
    pub locked: bool,
    pub posts: Vec<PostView>,
    pub page: u32,
    pub pages: u32,
    pub total: usize,
    pub fact: u32,
}

#[derive(Serialize)]
pub struct HitView {
    pub token: u32,
    pub fields: Vec<KV>,
    pub done: bool,
}

#[derive(Serialize)]
pub struct IncidentBrief {
    pub id: u32,
    pub company: String,
    pub category: &'static str,
    pub date: String,
    pub count_text: String,
}

#[derive(Serialize)]
pub struct HitGroup {
    pub incident: IncidentBrief,
    pub hits: Vec<HitView>,
    pub namesakes: Option<String>,
}

#[derive(Serialize)]
pub struct HitsView {
    pub key: String,
    pub filters: Vec<String>,
    pub groups: Vec<HitGroup>,
    pub total: usize,
    pub searched: usize,
    pub note: String,
}

#[derive(Serialize)]
pub struct IncidentView {
    pub id: u32,
    pub company: String,
    pub service: String,
    pub category: &'static str,
    pub cause: &'static str,
    pub occur: String,
    pub announce: String,
    pub announce_day: i32,
    pub circ: Option<String>,
    pub circulating: bool,
    pub count_text: String,
    pub pct_text: String,
    pub pct: f64,
    pub female: f64,
    pub age_dist: Vec<f64>,
    pub fields: Vec<&'static str>,
    pub persona_hit: bool,
    pub locality: Option<&'static str>,
    pub idol: Option<&'static str>,
    pub level: u8,
    pub past: bool,
}

#[derive(Serialize)]
pub struct Summary {
    pub id: u32,
    pub label: String,
    pub relation: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub birth: Option<String>,
    pub address: Option<String>,
    pub addr_level: u8,
    pub addr_label: &'static str,
    pub geo: Option<Geo>,
    pub geo_wrong: bool,
    pub phones: Vec<String>,
    pub emails: Vec<String>,
    pub sns: Vec<String>,
    pub school: Option<String>,
    pub work: Option<String>,
    pub extras: Vec<KV>,
    pub risk: u32,
    pub facts: usize,
    pub clues: usize,
    pub spent: i64,
    pub spent_text: String,
    pub addr_time_text: Option<String>,
    pub correct: Option<(usize, usize)>,
    pub alert: u32,
}

#[derive(Serialize)]
pub struct OptionItem {
    pub kind: &'static str,
    pub id: u32,
    pub label: String,
}

#[derive(Serialize)]
pub struct ActionAvail {
    pub key: &'static str,
    pub slots: Vec<Vec<OptionItem>>,
    pub ready: bool,
}

#[derive(Serialize)]
pub struct LogView {
    pub date: String,
    pub action: String,
    pub result: String,
    pub cost_text: String,
    pub ok: bool,
}

#[derive(Serialize)]
pub struct TargetView {
    pub summary: Summary,
    pub facts: Vec<FactView>,
    pub clues: Vec<ClueView>,
    pub actions: Vec<ActionAvail>,
    pub log: Vec<LogView>,
}

#[derive(Serialize)]
pub struct SourceEntryView {
    pub target: u32,
    pub label: String,
    pub relation: String,
    pub fields: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct SourceView {
    pub name: String,
    pub icon: &'static str,
    pub desc: String,
    pub entries: Vec<SourceEntryView>,
}

#[derive(Serialize)]
pub struct Stats {
    pub targets: usize,
    pub located: usize,
    pub exact: usize,
    pub facts: usize,
    pub incidents: usize,
    pub circulating: usize,
    pub self_leaks: usize,
    pub spent_text: String,
}

#[derive(Serialize)]
pub struct StateView {
    pub date: String,
    pub clock: String,
    pub day: i32,
    pub elapsed_days: i32,
    pub persona_title: &'static str,
    pub persona_icon: &'static str,
    pub persona_name: String,
    pub home: Geo,
    pub reveal: bool,
    pub stats: Stats,
    pub events: usize,
}

#[derive(Serialize)]
pub struct SelfLeak {
    pub incident: IncidentBrief,
    pub circulating: bool,
    pub fields: Vec<KV>,
}

#[derive(Serialize)]
pub struct SelfView {
    pub profile: Vec<KV>,
    pub leaks: Vec<SelfLeak>,
    pub exposed: Vec<&'static str>,
    pub risk: u32,
    pub accounts: Vec<KV>,
}

fn addr_level(f: &Fact) -> (u8, &'static str) {
    match f.init.field {
        Field::Address => (6, "番地まで特定"),
        Field::Town if f.init.aux > 0 => (5, "丁目まで"),
        Field::Town => (4, "町名まで"),
        Field::Postal => (4, "郵便番号まで"),
        Field::Station => (3, "最寄り駅まで"),
        Field::City => (2, "市区町村まで"),
        Field::Pref => (1, "都道府県まで"),
        _ => (0, "不明"),
    }
}

pub fn brief(inc: &Incident) -> IncidentBrief {
    IncidentBrief {
        id: inc.id,
        company: inc.company.clone(),
        category: CATEGORIES[inc.cat].label,
        date: date::fmt(inc.announce),
        count_text: format!("{}件", fmt_count(inc.count)),
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        s.chars().take(n).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

impl Sim {
    pub fn chain(&self, start: Parent) -> Vec<ChainStep> {
        let mut steps = Vec::new();
        let mut cur = Some(start);
        let mut guard = 0;
        while let Some(p) = cur {
            guard += 1;
            if guard > 16 {
                break;
            }
            match p {
                Parent::Fact(id) => {
                    let Some(f) = self.fact(id) else { break };
                    steps.push(ChainStep { action: f.via.clone(), label: f.init.field.label().to_string(), value: f.init.value.clone() });
                    cur = f.parents.first().copied();
                }
                Parent::Clue(id) => {
                    let Some(c) = self.clue(id) else { break };
                    steps.push(ChainStep { action: "SNSの投稿を見る → 写真を保存".into(), label: "写真".into(), value: c.caption.clone() });
                    cur = c.parent.map(Parent::Fact);
                }
            }
        }
        steps.reverse();
        steps
    }

    pub fn fact_view(&self, id: u32) -> Option<FactView> {
        let f = self.fact(id)?;
        let t = self.targets.iter().find(|t| t.facts.iter().any(|x| x.id == id))?;
        Some(FactView {
            id,
            field: f.init.field,
            label: f.init.field.label(),
            group: f.init.field.group(),
            value: f.init.value.clone(),
            sources: f.sources.clone(),
            date: date::fmt(f.day),
            via: f.via.clone(),
            note: f.note.clone(),
            cost_text: fmt_dur(f.cost),
            total: f.total,
            total_text: fmt_dur(f.total),
            chain: self.chain(Parent::Fact(id)),
            correct: self.reveal.then_some(f.init.subject == t.person),
        })
    }

    pub fn clue_view(&self, id: u32) -> Option<ClueView> {
        let c = self.clue(id)?;
        let t = self.targets.iter().find(|t| t.clues.iter().any(|x| x.id == id))?;
        Some(ClueView {
            id,
            platform: PLATFORMS[c.platform as usize].0,
            text: c.text.clone(),
            caption: c.caption.clone(),
            date: date::fmt(c.day),
            analyzed: c.analyzed,
            exif_checked: c.exif_checked,
            features: if c.analyzed { c.features.iter().map(|f| f.label()).collect() } else { Vec::new() },
            street_ok: c.analyzed && c.features.contains(&Feature::HouseView),
            chain: self.chain(Parent::Clue(id)),
            total_text: fmt_dur(c.total),
            correct: self.reveal.then_some(c.subject == t.person),
        })
    }

    pub fn account_view(&mut self, pid: u32, pf: u8, via: u32, how: u8, note: String) -> AccountView {
        let token = self.token(Ref::Account { pid, pf, via, how });
        let accessible = self.accessible(pid, pf);
        let a = self.w.p(pid).account(pf).expect("account");
        let known = self
            .targets
            .iter()
            .find(|t| t.facts.iter().any(|f| f.init.field == Field::Handle && f.init.aux == pf as i64 && f.init.key == a.handle))
            .map(|t| t.id);
        AccountView {
            token,
            platform: PLATFORMS[pf as usize].0,
            icon: PLATFORMS[pf as usize].1,
            handle: a.handle.clone(),
            display: a.display.clone(),
            bio: a.bio_text.clone(),
            public: a.public,
            accessible,
            note,
            known,
        }
    }

    pub fn summary(&self, t: usize) -> Summary {
        let tg = &self.targets[t];
        let first = |f: Field| tg.facts.iter().find(|x| x.init.field == f).map(|x| x.init.value.clone());
        let all = |fs: &[Field]| tg.facts.iter().filter(|x| fs.contains(&x.init.field)).map(|x| x.init.value.clone()).collect::<Vec<_>>();
        let best = tg
            .facts
            .iter()
            .filter(|f| f.init.geo.is_some() && f.init.field != Field::OldAddress)
            .min_by(|a, b| a.init.geo.unwrap().r.partial_cmp(&b.init.geo.unwrap().r).unwrap());
        let (lv, ll) = best.map(addr_level).unwrap_or((0, "不明"));
        let mut seen = Vec::new();
        let mut risk = 0;
        for f in &tg.facts {
            if !seen.contains(&f.init.field) {
                seen.push(f.init.field);
                risk += f.init.field.risk();
            }
        }
        let extra_fields = [
            Field::Routine, Field::OldAddress, Field::Family, Field::Face, Field::IdDoc, Field::Password, Field::Card, Field::Income,
            Field::Assets, Field::Health, Field::History, Field::Pet, Field::Idol, Field::Hobby,
        ];
        let extras = extra_fields
            .iter()
            .filter_map(|&f| first(f).map(|v| KV { label: f.label().to_string(), value: v }))
            .collect();
        let birth = first(Field::BirthDate).or_else(|| first(Field::BirthMd).map(|m| format!("{}（年不明）", m))).or_else(|| first(Field::Age));
        Summary {
            id: tg.id,
            label: tg.label.clone(),
            relation: tg.relation.clone(),
            name: first(Field::Name),
            alias: first(Field::Alias),
            birth,
            address: best.map(|f| f.init.value.clone()),
            addr_level: lv,
            addr_label: ll,
            geo: best.and_then(|f| f.init.geo),
            geo_wrong: self.reveal && best.is_some_and(|f| f.init.subject != tg.person),
            phones: all(&[Field::Mobile, Field::HomePhone]),
            emails: all(&[Field::Email]),
            sns: all(&[Field::Handle]),
            school: first(Field::School),
            work: first(Field::Workplace),
            extras,
            risk: risk.min(100),
            facts: tg.facts.len(),
            clues: tg.clues.len(),
            spent: tg.spent,
            spent_text: fmt_dur(tg.spent),
            addr_time_text: best.filter(|_| lv >= 4).map(|f| fmt_dur(f.total)),
            correct: self.reveal.then(|| (tg.facts.iter().filter(|f| f.init.subject == tg.person).count(), tg.facts.len())),
            alert: self.alert.get(&tg.person).copied().unwrap_or(0),
        }
    }

    pub fn target_view(&mut self, t: usize) -> Option<TargetView> {
        self.targets.get(t)?;
        let facts = self.targets[t].facts.iter().map(|f| f.id).collect::<Vec<_>>();
        let clues = self.targets[t].clues.iter().map(|c| c.id).collect::<Vec<_>>();
        let tg = &self.targets[t];
        let actions = ACTIONS
            .iter()
            .map(|a| {
                let slots: Vec<Vec<OptionItem>> = a.slots.iter().map(|s| slot_options(tg, s)).collect();
                ActionAvail { key: a.key, ready: slots.iter().all(|s| !s.is_empty()), slots }
            })
            .collect();
        let log = tg
            .log
            .iter()
            .rev()
            .map(|l| LogView { date: date::fmt(l.day), action: l.action.clone(), result: l.result.clone(), cost_text: fmt_dur(l.cost), ok: l.ok })
            .collect();
        Some(TargetView {
            summary: self.summary(t),
            facts: facts.into_iter().filter_map(|id| self.fact_view(id)).collect(),
            clues: clues.into_iter().filter_map(|id| self.clue_view(id)).collect(),
            actions,
            log,
        })
    }

    pub fn incident_view(&self, inc: &Incident) -> IncidentView {
        let mut comp = [0.0f64; 9];
        let mut fem = 0.0;
        let mut tot = 0.0;
        for b in 0..9 {
            let base = AGE_SHARE[b] * inc.age[b];
            comp[b] = base;
            fem += base * inc.female;
            tot += base;
        }
        let d = self.day();
        IncidentView {
            id: inc.id,
            company: inc.company.clone(),
            service: inc.service.clone(),
            category: CATEGORIES[inc.cat].label,
            cause: incident::cause_label(inc.cause),
            occur: date::fmt_ym(inc.occur),
            announce: date::fmt(inc.announce),
            announce_day: inc.announce,
            circ: inc.circ.filter(|&c| c <= d).map(date::fmt),
            circulating: inc.circ.is_some_and(|c| c <= d),
            count_text: format!("{}件", fmt_count(inc.count)),
            pct_text: fmt_pct(inc.pct),
            pct: inc.pct,
            female: if tot > 0.0 { fem / tot } else { 0.5 },
            age_dist: comp.iter().map(|c| if tot > 0.0 { c / tot } else { 0.0 }).collect(),
            fields: inc.fields.iter().map(|f| f.label()).collect(),
            persona_hit: inc.persona_hit,
            locality: inc.pref.map(|p| PREFS[p].0),
            idol: inc.idol.map(|g| IDOL_GROUPS[g].0),
            level: incident::severity(inc),
            past: inc.announce < self.start(),
        }
    }

    pub fn sources_view(&self) -> Vec<SourceView> {
        self.sources
            .iter()
            .map(|s| SourceView {
                name: s.name.clone(),
                icon: s.icon,
                desc: s.desc.clone(),
                entries: s
                    .entries
                    .iter()
                    .map(|(t, rel, fields)| SourceEntryView {
                        target: *t,
                        label: self.targets[*t as usize].label.clone(),
                        relation: rel.clone(),
                        fields: fields.iter().map(|f| f.label()).collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn state_view(&self) -> StateView {
        let sums: Vec<Summary> = (0..self.targets.len()).map(|t| self.summary(t)).collect();
        let me = self.w.p(self.persona);
        let a = self.w.addr(self.persona);
        let pinfo = PERSONAS.iter().find(|p| p.key == self.persona_key).unwrap_or(&PERSONAS[0]);
        let total: i64 = self.targets.iter().map(|t| t.spent).sum();
        let m = self.minute.rem_euclid(1440);
        StateView {
            date: date::fmt_wd(self.day()),
            clock: format!("{:02}:{:02}", m / 60, m % 60),
            day: self.day(),
            elapsed_days: self.day() - self.start(),
            persona_title: pinfo.title,
            persona_icon: pinfo.icon,
            persona_name: me.full_name(),
            home: Geo { lat: a.lat, lon: a.lon, r: 0.0 },
            reveal: self.reveal,
            stats: Stats {
                targets: self.targets.len(),
                located: sums.iter().filter(|s| s.addr_level >= 4).count(),
                exact: sums.iter().filter(|s| s.addr_level >= 6).count(),
                facts: sums.iter().map(|s| s.facts).sum(),
                incidents: self.incidents.len(),
                circulating: self.circulating().count(),
                self_leaks: self.incidents.iter().filter(|i| i.persona_hit).count(),
                spent_text: fmt_dur(total),
            },
            events: self.events.len(),
        }
    }

    pub fn self_view(&self) -> SelfView {
        let id = self.persona;
        let p = self.w.p(id);
        let now = self.day();
        let mut profile = Vec::new();
        for f in [
            Field::Name, Field::Kana, Field::BirthDate, Field::Address, Field::Mobile, Field::HomePhone, Field::Email, Field::School,
            Field::Workplace, Field::Family, Field::Password,
        ] {
            if let Some(v) = crate::facts::value(&self.w, id, f, 0, now) {
                profile.push(KV { label: f.label().into(), value: v.value });
            }
        }
        profile.push(KV { label: "職業".into(), value: p.occupation.into() });
        let accounts = p
            .accounts
            .iter()
            .map(|a| KV {
                label: PLATFORMS[a.platform as usize].0.into(),
                value: format!("@{}（{}）", a.handle, if a.public { "公開" } else { "鍵アカウント" }),
            })
            .collect();
        let mut exposed: Vec<Field> = Vec::new();
        let mut leaks = Vec::new();
        for inc in self.incidents.iter().filter(|i| i.persona_hit) {
            let rec = incident::record(&self.w, inc, id);
            for r in &rec {
                if !exposed.contains(&r.field) {
                    exposed.push(r.field);
                }
            }
            leaks.push(SelfLeak {
                incident: brief(inc),
                circulating: inc.circ.is_some_and(|c| c <= now),
                fields: rec.into_iter().map(|r| KV { label: r.field.label().into(), value: r.value }).collect(),
            });
        }
        leaks.reverse();
        exposed.sort();
        SelfView {
            profile,
            leaks,
            risk: exposed.iter().map(|f| f.risk()).sum::<u32>().min(100),
            exposed: exposed.iter().map(|f| f.label()).collect(),
            accounts,
        }
    }
}

fn slot_options(tg: &Target, s: &Slot) -> Vec<OptionItem> {
    if s.photo > 0 {
        tg.clues
            .iter()
            .filter(|c| s.photo == 1 || (c.analyzed && c.features.contains(&Feature::HouseView)))
            .map(|c| OptionItem { kind: "clue", id: c.id, label: format!("📷 {}（{}）", short(&c.text, 16), date::fmt(c.day)) })
            .collect()
    } else {
        tg.facts
            .iter()
            .filter(|f| s.fields.contains(&f.init.field))
            .map(|f| OptionItem { kind: "fact", id: f.id, label: format!("{}: {}", f.init.field.label(), short(&f.init.value, 28)) })
            .collect()
    }
}
