import "./styles.css";
import "leaflet/dist/leaflet.css";
import {
  actionCatalog,
  Game,
  initApi,
  listPersonas,
  type ActionInfo,
  type Geo,
  type PersonaInfo,
  type Resp,
} from "./api";
import { clear, el, type Ctx, type Panel } from "./store";
import { mountInvestigate } from "./investigate";
import { mountTimeline, mountIncidents, mountSelf, mountCollected, mountMap } from "./panels";

const app = document.getElementById("app")!;
let game: Game;
let catalog: ActionInfo[];
let seenEvents = 0; // 既読のイベント数（タイムラインのバッジ用）

const toasts = el("div", { class: "toasts" });
document.body.append(toasts);

function toast(msg: string, ok?: boolean) {
  const t = el("div", { class: `toast ${ok === true ? "ok" : ok === false ? "bad" : ""}`, text: msg });
  toasts.append(t);
  setTimeout(() => {
    t.style.opacity = "0";
    t.style.transition = "opacity .3s";
    setTimeout(() => t.remove(), 300);
  }, 4200);
}

// ---- スタート画面 ----
function startScreen() {
  const personas = listPersonas();
  const wrap = el("div", { class: "start" });
  wrap.append(
    el("div", { class: "hero" }, [
      el("h1", { text: "個人情報流出シミュレータ" }),
      el("p", {
        class: "muted",
        html:
          "企業からの個人情報流出が相次ぐ世界で時間を進め、ニュースを見守ります。そして、断片的な公開情報や流出データから一人の人物をどこまで特定できてしまうかを、<b>あなた自身の手で</b>体験します。",
      }),
      el("p", {
        class: "mute2",
        html:
          "⚠️ 登場する人物・企業・住所・電話番号・SNSはすべて<b>架空</b>です。これは手口の怖さと対策を学ぶための教材で、実在の誰かを調べる道具ではありません。",
      }),
    ]),
  );
  wrap.append(el("h3", { text: "あなた（ペルソナ）を選んでください", style: "margin-top:24px" }));
  wrap.append(
    el("p", { class: "muted", text: "選んだ立場に応じて「自分が持っている情報」（連絡網・名簿・SNSのフォローなど）が変わります。" }),
  );
  const grid = el("div", { class: "persona-grid" });
  for (const p of personas) grid.append(personaCard(p));
  wrap.append(grid);
  clear(app).append(wrap);
}

function personaCard(p: PersonaInfo): HTMLElement {
  return el("button", { class: "persona", onClick: () => begin(p.key) }, [
    el("div", { class: "ic", text: p.icon }),
    el("h3", { text: p.title }),
    el("div", { class: "chip sm", text: `📍 ${p.place}` }),
    el("p", { class: "muted", text: p.summary, style: "margin:8px 0 0" }),
  ]);
}

// ---- ゲーム開始 ----
const TABS: { key: string; icon: string; label: string; mount: (root: HTMLElement, ctx: Ctx) => Panel }[] = [
  { key: "timeline", icon: "🗞", label: "ニュース", mount: mountTimeline },
  { key: "incidents", icon: "📊", label: "流出事件", mount: mountIncidents },
  { key: "have", icon: "🗂", label: "自分の情報", mount: mountSources },
  { key: "investigate", icon: "🔎", label: "調査", mount: mountInvestigate },
  { key: "collected", icon: "📁", label: "取った情報", mount: mountCollected },
  { key: "map", icon: "🗺", label: "地図", mount: mountMap },
  { key: "self", icon: "🫣", label: "自分の流出", mount: mountSelf },
];

let panels: Record<string, Panel> = {};
let containers: Record<string, HTMLElement> = {};
let active = "timeline";
let badges: Record<string, HTMLElement> = {};
let clockEl: HTMLElement, statsEl: HTMLElement;

function begin(personaKey: string) {
  const seed = (Math.random() * 0xffffffff) >>> 0;
  game = new Game(personaKey, seed, 7.0);
  seenEvents = game.state().events; // 過去分は既読扱い
  buildLayout();
}

const ctx: Ctx = {
  get game() {
    return game;
  },
  get catalog() {
    return catalog;
  },
  openTarget(id: number) {
    switchTab("investigate");
    (panels.investigate as Panel & { focus?: (id: number) => void }).focus?.(id);
  },
  flyTo(geo: Geo, label: string) {
    switchTab("map");
    (panels.map as Panel & { flyTo?: (g: Geo, l: string) => void }).flyTo?.(geo, label);
  },
  refreshAll() {
    renderTopbar();
    for (const k of Object.keys(panels)) panels[k].refresh();
  },
  toast,
  pushEvents(resp: Resp) {
    for (const e of resp.events) {
      const icon = e.kind === "notify" ? "📧" : e.kind === "circulate" ? "🕶" : "🗞";
      toast(`${icon} ${e.title}`, e.kind === "notify" ? false : undefined);
    }
  },
};

function buildLayout() {
  clear(app);
  // トップバー
  const bar = el("div", { class: "topbar" });
  const inner = el("div", { class: "topbar-inner" });
  clockEl = el("div", { class: "clock" });
  const adv = el("div", { class: "advance" });
  const mk = (label: string, days: number) =>
    el("button", { class: "btn sm", text: label, onClick: () => advance(days) });
  adv.append(
    el("span", { class: "muted", text: "時間を進める:" }),
    mk("1日", 1),
    mk("1週", 7),
    mk("1か月", 30),
    mk("3か月", 91),
  );
  statsEl = el("div", { class: "topstats" });
  inner.append(clockEl, adv, el("div", { class: "grow" }), statsEl);
  bar.append(inner);

  // タブ
  const tabbar = el("div", { class: "tabs" });
  badges = {};
  for (const t of TABS) {
    const badge = el("span", { class: "badge", text: "" });
    badge.style.display = "none";
    badges[t.key] = badge;
    const btn = el("button", { class: "tab", dataset: { k: t.key }, onClick: () => switchTab(t.key) }, [
      el("span", { text: t.icon }),
      el("span", { text: t.label }),
      badge,
    ]);
    tabbar.append(btn);
  }
  bar.append(tabbar);

  const main = el("div", { class: "main" });
  panels = {};
  containers = {};
  for (const t of TABS) {
    const c = el("div", { class: "panel", dataset: { k: t.key } });
    containers[t.key] = c;
    main.append(c);
    panels[t.key] = t.mount(c, ctx);
  }
  app.append(bar, main);
  switchTab("timeline");
  renderTopbar();
}

function switchTab(key: string) {
  active = key;
  for (const t of TABS) {
    containers[t.key].classList.toggle("active", t.key === key);
    const btn = document.querySelector<HTMLElement>(`.tab[data-k="${t.key}"]`);
    btn?.classList.toggle("active", t.key === key);
  }
  panels[key].refresh();
  if (key === "timeline") {
    seenEvents = game.state().events;
    renderTopbar();
  }
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function advance(days: number) {
  const evs = game.advance(days);
  const notify = evs.filter((e) => e.kind === "notify");
  const circ = evs.filter((e) => e.kind === "circulate");
  if (notify.length) toast(`📧 あなた宛てに流出のお詫びが ${notify.length} 件届きました`, false);
  else if (circ.length) toast(`🕶 ${circ.length} 件の流出データが闇市場に出回り始めました`);
  else toast(`${days}日進めました（流出事件 ${evs.filter((e) => e.kind === "announce").length} 件）`);
  ctx.refreshAll();
}

function renderTopbar() {
  const s = game.state();
  clear(clockEl).append(
    el("span", { class: "d", text: `${s.persona_icon} ${s.date}` }),
    el("span", { class: "t", text: s.clock }),
  );
  const st = s.stats;
  clear(statsEl).append(
    stat("調査対象", st.targets),
    stat("住所判明", st.located),
    stat("取得情報", st.facts),
    stat("調査時間", st.spent_text),
    stat("流出事件", st.incidents),
    stat("自分の被害", st.self_leaks, st.self_leaks > 0),
  );
  // タイムラインの未読バッジ
  const unread = Math.max(0, s.events - seenEvents);
  const b = badges.timeline;
  if (b) {
    b.textContent = unread > 99 ? "99+" : String(unread);
    b.style.display = unread > 0 && active !== "timeline" ? "" : "none";
  }
  const sb = badges.self;
  if (sb) {
    sb.textContent = String(st.self_leaks);
    sb.className = "badge danger";
    sb.style.display = st.self_leaks > 0 ? "" : "none";
  }
}

function stat(label: string, value: string | number, danger = false): HTMLElement {
  return el("div", {}, [
    el("span", { text: label + " " }),
    el("b", { text: String(value), style: danger ? "color:var(--danger)" : "" }),
  ]);
}

// ---- 「自分の情報」タブ：ソース一覧（小さいので main 側で実装） ----
function mountSources(root: HTMLElement, c: Ctx): Panel {
  const refresh = () => {
    const sources = c.game.sources();
    clear(root);
    root.append(
      el("div", { class: "card" }, [
        el("h2", { text: "🗂 自分が持っている情報" }),
        el("p", {
          class: "muted",
          html:
            "あなたの手元にある名簿・連絡網・SNSのフォローなどです。ここに並ぶ一人ひとりが「調査対象」になります。名前を押すと調査画面が開きます。悪意ある第三者は、こうした<b>ごく普通の持ち物</b>を起点に個人を特定していきます。",
        }),
      ]),
    );
    for (const s of sources) {
      const box = el("div", { class: "card", style: "margin-top:12px" });
      box.append(
        el("div", { class: "row spread" }, [
          el("h3", { text: `${s.icon} ${s.name}`, style: "margin:0" }),
          el("span", { class: "chip", text: `${s.entries.length}人` }),
        ]),
        el("p", { class: "muted", text: s.desc, style: "margin:6px 0 10px" }),
      );
      const grid = el("div", { class: "grid cols2" });
      for (const e of s.entries) {
        grid.append(
          el("button", { class: "fieldcard", style: "border-left-color:var(--accent);text-align:left", onClick: () => c.openTarget(e.target) }, [
            el("div", { class: "row spread" }, [
              el("span", { class: "v", text: e.label }),
              el("span", { class: "chip sm", text: e.relation }),
            ]),
            el("div", { class: "muted", text: e.fields.join("・"), style: "font-size:.82em" }),
          ]),
        );
      }
      box.append(grid);
      root.append(box);
    }
  };
  return { refresh };
}

// ---- 起動 ----
initApi().then(() => {
  catalog = actionCatalog();
  startScreen();
}).catch((err) => {
  clear(app).append(el("div", { class: "start" }, [el("h2", { text: "読み込みに失敗しました" }), el("pre", { text: String(err) })]));
});
