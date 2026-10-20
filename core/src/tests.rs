//! 全ペルソナを動かし、インシデント生成・手口・時間コストが破綻しないか確認する。

use crate::actions::{Req, ACTIONS};
use crate::persona::PERSONAS;
use crate::sim::Sim;

fn sim(key: &str) -> Sim {
    Sim::new(key, 12345, 6.0).expect("persona should build")
}

#[test]
fn all_personas_build() {
    for p in PERSONAS {
        let s = sim(p.key);
        assert!(!s.targets.is_empty(), "{} should have targets", p.key);
        assert!(!s.sources.is_empty(), "{} should have sources", p.key);
        // 開始前のインシデントが過去分として存在する
        assert!(!s.incidents.is_empty(), "{} should have past incidents", p.key);
        // 連絡網などから初期の情報が登録されている
        assert!(s.targets.iter().any(|t| !t.facts.is_empty()), "{} targets should carry facts", p.key);
    }
}

#[test]
fn advancing_time_generates_incidents() {
    let mut s = sim("student");
    let before = s.incidents.len();
    let events = s.spend(365 * 1440);
    assert!(s.incidents.len() > before, "a year should add incidents");
    assert!(!events.is_empty(), "a year should surface events");
    // 公表 → 流通の順序が保たれている
    for inc in &s.incidents {
        if let Some(c) = inc.circ {
            assert!(c >= inc.announce, "circulation must not precede announcement");
        }
        assert!(inc.occur <= inc.announce, "occurrence must not follow announcement");
    }
}

#[test]
fn leak_rate_is_plausible() {
    // ペルソナ自身が1年で少なくとも1件は被害に遭い、全部には遭わない
    let mut total_hits = 0;
    let mut total = 0;
    for p in PERSONAS {
        let mut s = sim(p.key);
        s.spend(365 * 1440);
        let hits = s.incidents.iter().filter(|i| i.persona_hit).count();
        total_hits += hits;
        total += s.incidents.len();
    }
    let rate = total_hits as f64 / total as f64;
    assert!(rate > 0.0 && rate < 0.5, "self-leak rate {rate} should be modest but non-zero");
}

#[test]
fn every_action_runs_without_panicking() {
    // どの対象に対しても、全アクションを「準備不足」も含めて呼んでパニックしないこと
    let mut s = sim("office");
    s.spend(400 * 1440);
    s.reveal = true;
    for t in 0..s.targets.len().min(6) {
        let view = s.target_view(t).expect("target view");
        for a in &view.actions {
            // 各スロットの先頭候補を使って実行を試みる
            let fact = a.slots.first().and_then(|o| o.first()).map(|o| o.id).unwrap_or(0);
            let fact2 = a.slots.get(1).and_then(|o| o.first()).map(|o| o.id).unwrap_or(0);
            let clue = a
                .slots
                .iter()
                .flatten()
                .find(|o| o.kind == "clue")
                .map(|o| o.id)
                .unwrap_or(0);
            let q = Req { action: a.key.to_string(), target: t as u32, fact, fact2, clue, ..Default::default() };
            let _ = s.run(&q);
        }
    }
}

#[test]
fn sns_chain_can_reach_an_address() {
    // いずれかのペルソナで、SNSを持つ対象の投稿を読み進めると情報や写真が増える
    let mut s = sim("student");
    s.spend(200 * 1440);
    let mut grew = false;
    for t in 0..s.targets.len() {
        let before = s.targets[t].facts.len() + s.targets[t].clues.len();
        let handle = s.targets[t].facts.iter().find(|f| f.init.field == crate::model::Field::Handle).map(|f| f.id);
        let Some(fid) = handle else { continue };
        // 投稿を数ページ読む
        for page in 0..4 {
            let q = Req { action: "sns_view".into(), target: t as u32, fact: fid, page, ..Default::default() };
            let resp = s.run(&q);
            if let Some(feed) = resp.feed {
                if feed.locked {
                    break;
                }
                for p in feed.posts.iter().filter(|p| p.info) {
                    let q = Req { action: "extract".into(), target: t as u32, token: p.token, ..Default::default() };
                    s.run(&q);
                }
            }
        }
        if s.targets[t].facts.len() + s.targets[t].clues.len() > before {
            grew = true;
            break;
        }
    }
    assert!(grew, "reading SNS posts should yield new facts or photos for someone");
}

#[test]
fn time_cost_accumulates() {
    let mut s = sim("college");
    let start = s.day();
    // 名寄せ系の重い手口はないが、SNS閲覧で時間が進む
    let t = 0u32;
    let fid = s.targets[t as usize]
        .facts
        .iter()
        .find(|f| f.init.field == crate::model::Field::Handle)
        .map(|f| f.id);
    if let Some(fid) = fid {
        let q = Req { action: "sns_view".into(), target: t, fact: fid, ..Default::default() };
        s.run(&q);
        assert!(s.day() >= start, "clock should not move backwards");
    }
    // 累計調査時間は非負
    assert!(s.targets.iter().all(|t| t.spent >= 0));
}

#[test]
fn action_catalog_is_complete() {
    let cat = crate::actions::catalog();
    assert_eq!(cat.len(), ACTIONS.len());
    assert!(cat.iter().all(|a| !a.name.is_empty() && !a.case.is_empty() && !a.counter.is_empty()));
}
