//! 個人情報流出シミュレータのコアエンジン。
//! 登場する人物・企業・住所・電話番号・メールアドレスはすべて架空。

mod actions;
mod data;
mod date;
mod facts;
mod incident;
mod model;
mod persona;
mod seed;
mod sim;
mod sns;
mod view;
mod world;

use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use actions::Req;
pub use sim::Sim;

fn to_js<T: Serialize + ?Sized>(v: &T) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub struct Simulator {
    sim: Sim,
}

#[wasm_bindgen]
impl Simulator {
    #[wasm_bindgen(constructor)]
    pub fn new(persona: &str, seed: u32, intensity: f64) -> Result<Simulator, JsError> {
        Sim::new(persona, seed as u64, intensity).map(|sim| Simulator { sim }).ok_or_else(|| JsError::new("unknown persona"))
    }

    pub fn state(&self) -> JsValue {
        to_js(&self.sim.state_view())
    }

    /// 日数を進め、その間に起きたイベントを返す
    pub fn advance(&mut self, days: u32) -> JsValue {
        to_js(&self.sim.spend(days as i64 * 1440))
    }

    pub fn events(&self, from: usize) -> JsValue {
        let from = from.min(self.sim.events.len());
        to_js(&self.sim.events[from..])
    }

    pub fn incidents(&self) -> JsValue {
        let v: Vec<_> = self.sim.incidents.iter().rev().map(|i| self.sim.incident_view(i)).collect();
        to_js(&v)
    }

    pub fn incident(&self, id: u32) -> JsValue {
        match self.sim.incidents.get(id as usize) {
            Some(i) => to_js(&self.sim.incident_view(i)),
            None => JsValue::NULL,
        }
    }

    pub fn sources(&self) -> JsValue {
        to_js(&self.sim.sources_view())
    }

    pub fn targets(&self) -> JsValue {
        let v: Vec<_> = (0..self.sim.targets.len()).map(|t| self.sim.summary(t)).collect();
        to_js(&v)
    }

    pub fn target(&mut self, id: u32) -> JsValue {
        match self.sim.target_view(id as usize) {
            Some(v) => to_js(&v),
            None => JsValue::NULL,
        }
    }

    pub fn self_exposure(&self) -> JsValue {
        to_js(&self.sim.self_view())
    }

    pub fn run(&mut self, req: JsValue) -> Result<JsValue, JsError> {
        let q: Req = serde_wasm_bindgen::from_value(req).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(to_js(&self.sim.run(&q)))
    }

    pub fn set_reveal(&mut self, on: bool) {
        self.sim.reveal = on;
    }
}

#[wasm_bindgen]
pub fn personas() -> JsValue {
    to_js(&persona::PERSONAS)
}

#[wasm_bindgen]
pub fn action_catalog() -> JsValue {
    to_js(&actions::catalog())
}

#[cfg(test)]
mod tests;
