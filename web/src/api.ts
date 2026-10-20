// Rust コア（wasm）が返すデータの型定義と、Simulator の薄いラッパ。
// フィールド名は Rust 側の serde 出力そのまま（多くは snake_case）。

import initWasm, {
  Simulator,
  personas as wasmPersonas,
  action_catalog as wasmCatalog,
} from "./pkg/leak_sim.js";
import wasmUrl from "./pkg/leak_sim_bg.wasm?url";

// ---- enum 相当 ----
// Field は Rust 側で rename_all="snake_case"
export type Field = string; // "name" | "address" | "mobile" | ... （表示は label を使う）
export type Group = "identity" | "location" | "contact" | "social" | "sensitive";

export interface Geo {
  lat: number;
  lon: number;
  r: number; // 推定範囲の半径（m）。0 は自宅など確定点
}

export interface PersonaInfo {
  key: string;
  title: string;
  icon: string;
  place: string;
  summary: string;
}

export interface KV {
  label: string;
  value: string;
}

export interface ChainStep {
  action: string;
  label: string;
  value: string;
}

export interface SlotInfo {
  label: string;
  fields: string[]; // 人間可読のラベル
  photo: number; // 0:情報 1:写真 2:解析済みの家の写真
}

export interface ActionInfo {
  key: string;
  name: string;
  icon: string;
  cat: string;
  cat_label: string;
  time_text: string;
  desc: string;
  case: string; // 実際の事例
  counter: string; // 対策
  illegal: boolean;
  filters: boolean;
  slots: SlotInfo[];
}

export interface FactView {
  id: number;
  field: Field;
  label: string;
  group: Group;
  value: string;
  sources: string[];
  date: string;
  via: string;
  note: string;
  cost_text: string;
  total: number; // 分
  total_text: string;
  chain: ChainStep[];
  correct: boolean | null; // 答え合わせ時のみ
}

export interface ClueView {
  id: number;
  platform: string;
  text: string;
  caption: string;
  date: string;
  analyzed: boolean;
  exif_checked: boolean;
  features: string[];
  street_ok: boolean;
  chain: ChainStep[];
  total_text: string;
  correct: boolean | null;
}

export interface AccountView {
  token: number;
  platform: string;
  icon: string;
  handle: string;
  display: string;
  bio: string;
  public: boolean;
  accessible: boolean;
  note: string;
  known: number | null; // 既存ターゲットID
}

export interface PostView {
  token: number;
  date: string;
  text: string;
  photo: string | null; // 写真の説明
  info: boolean; // 手がかりを含む
  done: boolean; // 記録済み
}

export interface FeedView {
  profile_token: number;
  profile_info: boolean;
  profile_done: boolean;
  account: AccountView;
  locked: boolean;
  posts: PostView[];
  page: number;
  pages: number;
  total: number;
  fact: number;
}

export interface HitView {
  token: number;
  fields: KV[];
  done: boolean;
}

export interface IncidentBrief {
  id: number;
  company: string;
  category: string;
  date: string;
  count_text: string;
}

export interface HitGroup {
  incident: IncidentBrief;
  hits: HitView[];
  namesakes: string | null;
}

export interface HitsView {
  key: string;
  filters: string[];
  groups: HitGroup[];
  total: number;
  searched: number;
  note: string;
}

export interface OptionItem {
  kind: "fact" | "clue";
  id: number;
  label: string;
}

export interface ActionAvail {
  key: string;
  slots: OptionItem[][];
  ready: boolean;
}

export interface LogView {
  date: string;
  action: string;
  result: string;
  cost_text: string;
  ok: boolean;
}

export interface Summary {
  id: number;
  label: string;
  relation: string;
  name: string | null;
  alias: string | null;
  birth: string | null;
  address: string | null;
  addr_level: number; // 0..6
  addr_label: string;
  geo: Geo | null;
  geo_wrong: boolean;
  phones: string[];
  emails: string[];
  sns: string[];
  school: string | null;
  work: string | null;
  extras: KV[];
  risk: number; // 0..100
  facts: number;
  clues: number;
  spent: number; // 分
  spent_text: string;
  addr_time_text: string | null; // 住所特定までにかかった時間
  correct: [number, number] | null; // [正しい情報数, 全情報数]
  alert: number;
}

export interface TargetView {
  summary: Summary;
  facts: FactView[];
  clues: ClueView[];
  actions: ActionAvail[];
  log: LogView[];
}

export interface SourceEntryView {
  target: number;
  label: string;
  relation: string;
  fields: string[];
}

export interface SourceView {
  name: string;
  icon: string;
  desc: string;
  entries: SourceEntryView[];
}

export interface IncidentView {
  id: number;
  company: string;
  service: string;
  category: string;
  cause: string;
  occur: string;
  announce: string;
  announce_day: number;
  circ: string | null;
  circulating: boolean;
  count_text: string;
  pct_text: string;
  pct: number;
  female: number;
  age_dist: number[]; // 9 区分
  fields: string[];
  persona_hit: boolean;
  locality: string | null;
  idol: string | null;
  level: number; // 1..5
  past: boolean;
}

export interface EventView {
  day: number;
  date: string;
  kind: "announce" | "notify" | "circulate";
  title: string;
  body: string;
  incident: number | null;
  level: number;
}

export interface Stats {
  targets: number;
  located: number;
  exact: number;
  facts: number;
  incidents: number;
  circulating: number;
  self_leaks: number;
  spent_text: string;
}

export interface StateView {
  date: string;
  clock: string;
  day: number;
  elapsed_days: number;
  persona_title: string;
  persona_icon: string;
  persona_name: string;
  home: Geo;
  reveal: boolean;
  stats: Stats;
  events: number;
}

export interface SelfLeak {
  incident: IncidentBrief;
  circulating: boolean;
  fields: KV[];
}

export interface SelfView {
  profile: KV[];
  leaks: SelfLeak[];
  exposed: string[];
  risk: number;
  accounts: KV[];
}

export interface Req {
  action: string;
  target?: number;
  fact?: number;
  fact2?: number;
  clue?: number;
  token?: number;
  page?: number;
  use_pref?: boolean;
  use_year?: boolean;
}

export interface Resp {
  ok: boolean;
  message: string;
  detail: string[];
  minutes: number;
  time_text: string;
  new_facts: FactView[];
  new_clues: ClueView[];
  feed: FeedView | null;
  accounts: AccountView[] | null;
  hits: HitsView | null;
  new_target: number | null;
  events: EventView[];
  target: number | null;
}

// ---- ラッパ ----

let ready: Promise<void> | null = null;

export function initApi(): Promise<void> {
  if (!ready) {
    ready = initWasm({ module_or_path: wasmUrl }).then(() => undefined);
  }
  return ready;
}

export function listPersonas(): PersonaInfo[] {
  return wasmPersonas() as PersonaInfo[];
}

export function actionCatalog(): ActionInfo[] {
  return wasmCatalog() as ActionInfo[];
}

/** シミュレーション本体。wasm の Simulator を型付きで包む。 */
export class Game {
  private sim: Simulator;
  constructor(persona: string, seed: number, intensity: number) {
    this.sim = new Simulator(persona, seed >>> 0, intensity);
  }
  state(): StateView {
    return this.sim.state() as StateView;
  }
  advance(days: number): EventView[] {
    return this.sim.advance(days) as EventView[];
  }
  events(from: number): EventView[] {
    return this.sim.events(from) as EventView[];
  }
  incidents(): IncidentView[] {
    return this.sim.incidents() as IncidentView[];
  }
  incident(id: number): IncidentView | null {
    return this.sim.incident(id) as IncidentView | null;
  }
  sources(): SourceView[] {
    return this.sim.sources() as SourceView[];
  }
  targets(): Summary[] {
    return this.sim.targets() as Summary[];
  }
  target(id: number): TargetView | null {
    return this.sim.target(id) as TargetView | null;
  }
  selfExposure(): SelfView {
    return this.sim.self_exposure() as SelfView;
  }
  run(req: Req): Resp {
    return this.sim.run(req) as Resp;
  }
  setReveal(on: boolean): void {
    this.sim.set_reveal(on);
  }
}
