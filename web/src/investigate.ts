// 「調査」タブ。調査対象の一覧と、選んだ対象の詳細（サマリー・手口の実行・取得情報・手がかり写真・ログ）を描画する。
// 手口の実行結果（SNSの投稿・アカウント候補・流出データの名寄せ）は「結果エリア」に固定して表示し、
// refreshAll による再描画でも消えないよう、対象ごとにモジュール内で保持する。
// 登場する人物・データはすべて架空（教育目的）。

import type {
  AccountView,
  ActionAvail,
  ActionInfo,
  ChainStep,
  ClueView,
  FactView,
  FeedView,
  HitView,
  HitsView,
  OptionItem,
  PostView,
  Req,
  Resp,
  Summary,
  TargetView,
} from "./api";
import {
  GROUP_COLOR,
  GROUP_LABEL,
  addrLevelLabel,
  clear,
  el,
  riskColor,
  riskLabel,
  type Ctx,
  type Panel,
} from "./store";

// ---- 定数・小物 ----

const INPUT_STYLE =
  "background:var(--bg2);color:var(--fg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;font:inherit;width:100%";
const SCROLL_MARGIN = "scroll-margin-top:120px";
const NARROW = "(max-width: 759px)";

/** 画面上の結果から続けて行う「記録」系の操作（結果エリアを置き換えず、実行後もスクロールしない） */
const RECORD_ACTIONS = new Set([
  "extract",
  "extract_profile",
  "link_account",
  "link_hit",
  "new_target_account",
  "new_target_hit",
]);

type BoxKind = "warn" | "danger" | "info";
const BOX_STYLE: Record<BoxKind, string> = {
  warn: "background:#2d1c0d;border:1px solid #5a3a1a;color:#ffb27a",
  danger: "background:#3a1518;border:1px solid #5a2024;color:#ff9ca0",
  info: "background:var(--bg2);border:1px solid var(--line);color:var(--fg-dim)",
};

function box(kind: BoxKind, text: string): HTMLElement {
  return el("div", { text, style: `${BOX_STYLE[kind]};border-radius:8px;padding:8px 10px;font-size:.9em` });
}

function chip(text: string, cls = "", style = ""): HTMLElement {
  return el("span", { class: `chip ${cls}`.trim(), text, style });
}

function button(text: string, onClick: () => void, cls = "btn sm", disabled = false): HTMLButtonElement {
  return el("button", { type: "button", class: cls, text, onClick, disabled });
}

function meter(risk: number): HTMLElement {
  const w = Math.max(0, Math.min(100, risk));
  return el("div", { class: "meter" }, [el("span", { style: `width:${w}%;background:${riskColor(risk)}` })]);
}

function kvItem(label: string, value: string | Node): HTMLElement {
  return el("div", { style: "min-width:0" }, [
    el("div", { class: "muted", text: label, style: "font-size:.78em" }),
    typeof value === "string" ? el("div", { text: value, style: "overflow-wrap:anywhere" }) : value,
  ]);
}

function optValue(o: Pick<OptionItem, "kind" | "id">): string {
  return `${o.kind}:${o.id}`;
}

// ---- モーダル ----

function openModal(title: string, body: Node): { close(): void } {
  const bg = el("div", { class: "modal-bg" });
  const onKey = (e: KeyboardEvent): void => {
    if (e.key === "Escape") close();
  };
  const close = (): void => {
    bg.remove();
    document.removeEventListener("keydown", onKey);
  };
  bg.addEventListener("click", (e) => {
    if (e.target === bg) close();
  });
  document.addEventListener("keydown", onKey);
  bg.append(
    el("div", { class: "modal", role: "dialog" }, [
      el("header", {}, [
        el("h3", { text: title, style: "margin:0" }),
        el("button", { type: "button", class: "x", text: "×", "aria-label": "閉じる", onClick: close }),
      ]),
      el("div", { class: "body" }, [body]),
    ]),
  );
  document.body.append(bg);
  return { close };
}

function confirmModal(title: string, message: string, okText: string, onOk: () => void): void {
  const body = el("div", { class: "stack" });
  const m = openModal(title, body);
  body.append(
    box("danger", message),
    el("div", { class: "row", style: "justify-content:flex-end" }, [
      button("やめる", () => m.close(), "btn"),
      button(okText, () => {
        m.close();
        onOk();
      }, "btn danger"),
    ]),
  );
}

// ---- 由来チェーン ----

function chainNode(steps: ChainStep[]): HTMLElement {
  if (steps.length === 0) return el("div", { class: "muted", text: "由来の記録がありません。" });
  const wrap = el("div", { class: "row wrap", style: "align-items:stretch;gap:6px" });
  steps.forEach((s, i) => {
    if (i > 0) wrap.append(el("span", { text: "→", style: "align-self:center;color:var(--fg-dim);font-size:1.2em" }));
    wrap.append(
      el("div", { class: "fieldcard", style: "border-left-color:var(--accent);min-width:120px;max-width:100%" }, [
        el("div", { class: "muted", text: s.action, style: "font-size:.75em" }),
        el("div", { class: "muted", text: s.label, style: "font-size:.8em;margin-top:2px" }),
        el("div", { class: "v", text: s.value, style: "overflow-wrap:anywhere" }),
      ]),
    );
  });
  return wrap;
}

function openFactModal(f: FactView): void {
  const body = el("div", { class: "stack" });
  body.append(el("div", { class: "v", text: f.value, style: "font-size:1.15em;overflow-wrap:anywhere" }));
  const meta = el("div", { class: "grid cols2" });
  if (f.sources.length) meta.append(kvItem("出どころ", f.sources.join("、")));
  if (f.via) meta.append(kvItem("手口", f.via));
  meta.append(kvItem("取得した日", f.date));
  meta.append(kvItem("この情報にかかった時間", f.cost_text));
  meta.append(kvItem("出どころを辿った累計時間", f.total_text));
  body.append(meta);
  if (f.note) body.append(box("info", `メモ: ${f.note}`));
  if (f.correct === false) body.append(box("danger", "✗ 別人の情報かもしれません（答え合わせ: この対象本人のものではない）。"));
  body.append(el("h4", { text: "この情報にたどり着いた経路", style: "margin:6px 0 0" }), chainNode(f.chain));
  openModal(`${f.label} の由来`, body);
}

function openClueModal(c: ClueView): void {
  const body = el("div", { class: "stack" });
  body.append(
    el("div", { text: c.caption, style: "overflow-wrap:anywhere" }),
    kvItem("出どころを辿った累計時間", c.total_text),
    el("h4", { text: "この写真にたどり着いた経路", style: "margin:6px 0 0" }),
    chainNode(c.chain),
  );
  openModal("写真の由来", body);
}

// ---- 取得情報カード ----

function factCard(f: FactView): HTMLElement {
  const color = GROUP_COLOR[f.group] ?? "var(--accent)";
  const top: (Node | string)[] = [el("span", { class: "muted", text: f.label, style: "font-size:.8em" })];
  if (f.correct === false) top.push(chip("✗ 別人の情報かも", "sm tag-illegal"));
  else if (f.correct === true) top.push(el("span", { text: "✓", style: "color:var(--ok)", title: "正しい情報" }));
  const kids: (Node | string)[] = [
    el("div", { class: "row spread" }, top),
    el("div", { class: "v", text: f.value, style: "overflow-wrap:anywhere" }),
  ];
  if (f.sources.length) kids.push(el("div", { class: "muted", text: `出どころ: ${f.sources.join("、")}`, style: "font-size:.78em" }));
  if (f.via) kids.push(el("div", { class: "muted", text: `手口: ${f.via}`, style: "font-size:.78em" }));
  kids.push(el("div", { class: "mute2", text: `⏱ ${f.cost_text}（累計 ${f.total_text}）`, style: "font-size:.78em" }));
  return el(
    "button",
    {
      type: "button",
      class: "fieldcard",
      style: `border-left-color:${color};text-align:left;color:var(--fg);width:100%`,
      title: "クリックで由来を表示",
      onClick: () => openFactModal(f),
    },
    kids,
  );
}

// ---- サマリーカード ----

function summaryCard(s: Summary, ctx: Ctx): HTMLElement {
  const card = el("div", { class: "card stack" });

  // 見出し
  const chips: (Node | string)[] = [chip(s.relation)];
  if (s.name && s.name !== s.label) chips.push(chip(`名前: ${s.name}`));
  if (s.alias && s.alias !== s.label) chips.push(chip(`別名: ${s.alias}`));
  const headRight: (Node | string)[] = [];
  const geo = s.geo;
  if (geo !== null) headRight.push(button("🗺 地図で見る", () => ctx.flyTo(geo, s.label)));
  card.append(
    el("div", { class: "row spread wrap" }, [
      el("div", {}, [
        el("h2", { text: s.label, style: "margin:0" }),
        el("div", { class: "row wrap", style: "margin-top:4px" }, chips),
      ]),
      el("div", { class: "row" }, headRight),
    ]),
  );

  // 危険度
  card.append(
    el("div", { class: "row" }, [
      el("span", { class: "muted", text: "危険度", style: "font-size:.85em" }),
      el("div", { class: "grow" }, [meter(s.risk)]),
      el("b", { text: `${s.risk}（${riskLabel(s.risk)}）`, style: `color:${riskColor(s.risk)};white-space:nowrap` }),
    ]),
  );

  // 判明している情報
  const grid = el("div", { class: "grid cols2" });
  if (s.name) grid.append(kvItem("名前", s.name));
  if (s.alias) grid.append(kvItem("別名", s.alias));
  if (s.birth) grid.append(kvItem("生年月日", s.birth));
  const addrKids: (Node | string)[] = [
    el("div", { text: s.address ?? "不明", style: "overflow-wrap:anywhere" }),
    el("div", { class: "row wrap", style: "gap:4px;margin-top:2px" }, [
      chip(`判明レベル: ${addrLevelLabel(s.addr_level)}`, "sm"),
      chip(s.addr_label, "sm"),
    ]),
  ];
  if (s.addr_time_text) addrKids.push(el("div", { class: "muted", text: `住所特定まで ${s.addr_time_text}`, style: "font-size:.8em" }));
  grid.append(kvItem("住所", el("div", {}, addrKids)));
  if (s.phones.length) grid.append(kvItem("電話番号", s.phones.join("、")));
  if (s.emails.length) grid.append(kvItem("メール", s.emails.join("、")));
  if (s.sns.length) grid.append(kvItem("SNS", s.sns.join("、")));
  if (s.school) grid.append(kvItem("学校", s.school));
  if (s.work) grid.append(kvItem("勤務先", s.work));
  for (const kv of s.extras) grid.append(kvItem(kv.label, kv.value));
  card.append(grid);

  card.append(
    el("div", {
      class: "muted",
      text: `取得情報 ${s.facts}件・手がかり写真 ${s.clues}枚・調査時間 ${s.spent_text}`,
      style: "font-size:.82em",
    }),
  );

  // 答え合わせ・警告
  if (s.correct !== null) {
    const [ok, all] = s.correct;
    card.append(chip(`答え合わせ: 正答 ${ok}/${all}`, "", ok < all ? "color:var(--warn);align-self:flex-start" : "color:var(--ok);align-self:flex-start"));
  }
  if (s.geo_wrong) card.append(box("danger", "⚠ 別人の住所を掴んでいます（同姓同名などの取り違え）。"));
  if (s.alert > 0) {
    card.append(
      box(
        s.alert >= 3 ? "danger" : "warn",
        s.alert >= 3
          ? `⚠ 相手の警戒度 ${s.alert}/3：強く警戒されており、対人の手口はもう効きません。`
          : `⚠ 相手の警戒度 ${s.alert}/3：不審に思われ始めています（手口の成功率が下がり、3で効かなくなります）。`,
      ),
    );
  }
  return card;
}

// ---- 結果エリアの状態 ----

interface ResultState {
  message: string;
  ok: boolean;
  detail: string[];
  minutes: number;
  timeText: string;
  newFacts: FactView[];
  newClues: ClueView[];
  feed: FeedView | null;
  accounts: AccountView[] | null;
  hits: HitsView | null;
  /** accounts を得た手口のキー（注意書きの出し分けに使う） */
  panelAction: string;
  /** この画面内で記録済みにした操作（`${action}:${token}`） */
  done: Set<string>;
}

interface FormState {
  slots: string[];
  usePref: boolean;
  useYear: boolean;
}

// ---- メイン ----

export function mountInvestigate(root: HTMLElement, ctx: Ctx): Panel & { focus(id: number): void } {
  let selected: number | null = null;
  let query = "";
  let sortRisk = false;
  let activeCat = "";
  let currentView: TargetView | null = null;
  let resultNode: HTMLElement | null = null;
  let actionsHost: HTMLElement | null = null;
  const results = new Map<number, ResultState>();
  const forms = new Map<string, FormState>();

  // ---- 左カラム（一覧） ----
  const countEl = el("span", { class: "muted", style: "font-size:.82em" });
  const searchBox = el("input", { type: "search", placeholder: "名前・別名で検索", style: INPUT_STYLE });
  const pillOrder = el("button", { type: "button", class: "pill active", text: "登録順" });
  const pillRisk = el("button", { type: "button", class: "pill", text: "危険度の高い順" });
  const listEl = el("div", { class: "stack scroll" });
  const detailEl = el("div", { class: "stack", style: `gap:12px;min-width:0;${SCROLL_MARGIN}` });

  searchBox.addEventListener("input", () => {
    query = searchBox.value.trim().toLowerCase();
    renderList();
  });
  pillOrder.addEventListener("click", () => {
    sortRisk = false;
    renderList();
  });
  pillRisk.addEventListener("click", () => {
    sortRisk = true;
    renderList();
  });

  const left = el("div", { class: "card stack" }, [
    el("div", { class: "row spread" }, [el("h3", { text: "🎯 調査対象", style: "margin:0" }), countEl]),
    searchBox,
    el("div", { class: "pill-tabs" }, [pillOrder, pillRisk]),
    listEl,
  ]);
  root.append(el("div", { class: "grid side" }, [left, detailEl]));

  function renderList(): void {
    const all = ctx.game.targets();
    let items = all.filter(
      (s) =>
        query === "" ||
        s.label.toLowerCase().includes(query) ||
        (s.name ?? "").toLowerCase().includes(query) ||
        (s.alias ?? "").toLowerCase().includes(query),
    );
    if (sortRisk) items = [...items].sort((a, b) => b.risk - a.risk || a.id - b.id);
    pillOrder.classList.toggle("active", !sortRisk);
    pillRisk.classList.toggle("active", sortRisk);
    countEl.textContent = `${items.length}/${all.length}件`;
    clear(listEl);
    if (items.length === 0) {
      listEl.append(el("div", { class: "empty", text: all.length === 0 ? "調査対象がまだありません" : "該当する対象がありません" }));
      return;
    }
    for (const s of items) listEl.append(targetButton(s));
  }

  function targetButton(s: Summary): HTMLElement {
    const on = s.id === selected;
    const style =
      `flex-shrink:0;width:100%;text-align:left;color:var(--fg);display:block;` +
      (on ? `background:var(--bg3);border-color:var(--accent);` : "") +
      `border-left-color:${riskColor(s.risk)}`;
    return el(
      "button",
      { type: "button", class: "fieldcard", style, dataset: { id: String(s.id) }, onClick: () => focus(s.id) },
      [
        el("div", { class: "row spread" }, [
          el("span", { class: "v", text: s.label, style: "overflow-wrap:anywhere" }),
          chip(s.relation, "sm"),
        ]),
        el("div", { class: "row", style: "margin:4px 0" }, [
          el("div", { class: "grow" }, [meter(s.risk)]),
          el("span", { class: "num", text: String(s.risk), style: `font-size:.8em;color:${riskColor(s.risk)}` }),
        ]),
        el("div", { class: "row wrap", style: "gap:4px" }, [
          chip(`📍 ${addrLevelLabel(s.addr_level)}`, "sm"),
          el("span", { class: "muted", text: s.addr_label, style: "font-size:.78em" }),
        ]),
        el("div", { class: "mute2", text: `情報 ${s.facts}件・⏱ ${s.spent_text}`, style: "font-size:.78em;margin-top:2px" }),
      ],
    );
  }

  // ---- 右カラム（詳細） ----
  function renderDetail(): void {
    clear(detailEl);
    resultNode = null;
    actionsHost = null;
    currentView = null;
    if (selected === null) {
      detailEl.append(el("div", { class: "card empty", text: "左の一覧から調査対象を選んでください" }));
      return;
    }
    const tv = ctx.game.target(selected);
    if (tv === null) {
      selected = null;
      detailEl.append(el("div", { class: "card empty", text: "調査対象が見つかりません" }));
      return;
    }
    currentView = tv;
    const id = selected;
    detailEl.append(summaryCard(tv.summary, ctx));
    const rs = results.get(id);
    if (rs) {
      resultNode = resultCard(id, rs);
      detailEl.append(resultNode);
    }
    actionsHost = el("div", { class: "card stack" });
    fillActions(actionsHost, tv, id);
    detailEl.append(actionsHost, factsPanel(tv), cluesPanel(tv, id), logPanel(tv));
  }

  function focus(id: number): void {
    const changed = selected !== id;
    selected = id;
    renderList();
    renderDetail();
    if (changed && window.matchMedia(NARROW).matches) detailEl.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function refresh(): void {
    const visible = root.getClientRects().length > 0;
    const y = window.scrollY;
    renderList();
    renderDetail();
    if (visible) window.scrollTo(0, y);
  }

  // ---- 手口の実行 ----

  function applyResult(origin: number, req: Req, resp: Resp): void {
    const prev = results.get(origin);
    const fresh = resp.feed != null || resp.accounts != null || resp.hits != null;
    // 新しい表示物が来ない限り、直前のフィード・候補一覧・名寄せ結果は残す
    const base = !fresh && prev !== undefined ? prev : undefined;
    const done = base ? base.done : new Set<string>();
    if (resp.ok && req.token !== undefined && RECORD_ACTIONS.has(req.action)) done.add(`${req.action}:${req.token}`);
    results.set(origin, {
      message: resp.message,
      ok: resp.ok,
      detail: resp.detail ?? [],
      minutes: resp.minutes,
      timeText: resp.time_text,
      newFacts: resp.new_facts ?? [],
      newClues: resp.new_clues ?? [],
      feed: fresh ? (resp.feed ?? null) : (base?.feed ?? null),
      accounts: fresh ? (resp.accounts ?? null) : (base?.accounts ?? null),
      hits: fresh ? (resp.hits ?? null) : (base?.hits ?? null),
      panelAction: fresh ? req.action : (base?.panelAction ?? ""),
      done,
    });
  }

  function exec(req: Req): void {
    const origin = selected;
    if (origin === null) return;
    const resp = ctx.game.run(req);
    applyResult(origin, req, resp);
    ctx.toast(resp.message, resp.ok);
    ctx.pushEvents(resp);
    ctx.refreshAll();
    if (req.action.startsWith("new_target")) {
      if (resp.ok && resp.new_target != null) focus(resp.new_target);
      return;
    }
    if (!RECORD_ACTIONS.has(req.action)) resultNode?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function actionLabel(key: string): string {
    const a = ctx.catalog.find((x) => x.key === key);
    return a ? `${a.name}（${a.time_text}）` : key;
  }

  // ---- 手口パネル ----

  function formFor(target: number, key: string): FormState {
    const k = `${target}:${key}`;
    let f = forms.get(k);
    if (!f) {
      f = { slots: [], usePref: false, useYear: false };
      forms.set(k, f);
    }
    return f;
  }

  function categories(): { key: string; label: string }[] {
    const cats: { key: string; label: string }[] = [];
    for (const a of ctx.catalog) if (!cats.some((c) => c.key === a.cat)) cats.push({ key: a.cat, label: a.cat_label });
    return cats;
  }

  function fillActions(host: HTMLElement, tv: TargetView, target: number): void {
    clear(host);
    const avail = new Map<string, ActionAvail>(tv.actions.map((a) => [a.key, a]));
    const cats = categories();
    if (!cats.some((c) => c.key === activeCat)) activeCat = cats.length ? cats[0].key : "";

    const tabs = el("div", { class: "pill-tabs" });
    for (const c of cats) {
      const inCat = ctx.catalog.filter((a) => a.cat === c.key);
      const ready = inCat.filter((a) => avail.get(a.key)?.ready === true).length;
      tabs.append(
        el("button", {
          type: "button",
          class: `pill ${c.key === activeCat ? "active" : ""}`,
          text: `${c.label}（${ready}/${inCat.length}）`,
          onClick: () => {
            activeCat = c.key;
            fillActions(host, tv, target);
          },
        }),
      );
    }

    const grid = el("div", { class: "grid cols2" });
    for (const a of ctx.catalog.filter((x) => x.cat === activeCat)) grid.append(actionCard(a, avail.get(a.key), target));

    host.append(
      el("h3", { text: "🛠 手口（アクション）", style: "margin:0" }),
      el("div", {
        class: "muted",
        text: "手口を実行するとゲーム内の時間が進みます。括弧内は「いま実行できる手口 / 全体」。必要な情報が手元にないと実行できません。",
        style: "font-size:.85em",
      }),
      tabs,
      grid,
    );
  }

  function actionCard(a: ActionInfo, av: ActionAvail | undefined, target: number): HTMLElement {
    const ready = av?.ready === true;
    const form = formFor(target, a.key);
    const selects: (HTMLSelectElement | undefined)[] = [];
    const missing: string[] = [];
    const slotRows: HTMLElement[] = [];

    a.slots.forEach((slot, i) => {
      const opts = av?.slots[i] ?? [];
      if (opts.length === 0) {
        missing.push(slot.fields.length ? slot.fields.join("・") : slot.label);
        return;
      }
      const sel = el("select", { style: INPUT_STYLE });
      for (const o of opts) sel.append(el("option", { value: optValue(o), text: o.label }));
      const saved = form.slots[i];
      if (saved !== undefined && opts.some((o) => optValue(o) === saved)) sel.value = saved;
      sel.addEventListener("change", () => {
        form.slots[i] = sel.value;
      });
      selects[i] = sel;
      slotRows.push(
        el("label", { style: "display:flex;flex-direction:column;gap:2px" }, [
          el("span", { class: "muted", text: slot.label, style: "font-size:.8em" }),
          sel,
        ]),
      );
    });

    // 絞り込み（流出データの名寄せのみ）
    const filterRows: HTMLElement[] = [];
    let prefBox: HTMLInputElement | undefined;
    let yearBox: HTMLInputElement | undefined;
    if (a.filters) {
      const pb = el("input", { type: "checkbox" });
      const yb = el("input", { type: "checkbox" });
      pb.checked = form.usePref;
      yb.checked = form.useYear;
      pb.addEventListener("change", () => {
        form.usePref = pb.checked;
      });
      yb.addEventListener("change", () => {
        form.useYear = yb.checked;
      });
      prefBox = pb;
      yearBox = yb;
      filterRows.push(
        el("label", { class: "row", style: "gap:6px;font-size:.88em" }, [pb, el("span", { text: "居住地で絞り込む" })]),
        el("label", { class: "row", style: "gap:6px;font-size:.88em" }, [yb, el("span", { text: "生まれ年で絞り込む" })]),
        el("div", {
          class: "mute2",
          text: "判明している居住地・生まれ年を使って、同姓同名の別人を減らします。",
          style: "font-size:.78em",
        }),
      );
    }

    const buildReq = (): Req => {
      const req: Req = { action: a.key, target };
      for (let i = 0; i < a.slots.length; i++) {
        const sel = selects[i];
        const o = sel ? av?.slots[i].find((x) => optValue(x) === sel.value) : undefined;
        if (!o) continue;
        if (o.kind === "clue") req.clue = o.id;
        else if (req.fact === undefined) req.fact = o.id;
        else req.fact2 = o.id;
      }
      if (a.filters) {
        req.use_pref = prefBox?.checked ?? false;
        req.use_year = yearBox?.checked ?? false;
      }
      return req;
    };

    const run = (): void => {
      const req = buildReq();
      if (a.illegal) {
        confirmModal(
          `「${a.name}」は法律に触れる手口です`,
          "これは実際には犯罪です。教材上の擬似実行として進めますか？（実在の誰かに対しては絶対に行わないでください）",
          "擬似実行する",
          () => exec(req),
        );
      } else {
        exec(req);
      }
    };

    const titleChips: (Node | string)[] = [chip(`⏱ ${a.time_text}`, "sm")];
    if (a.illegal) titleChips.push(chip("⚖ 法律に触れる手口", "sm tag-illegal"));

    const kids: (Node | string)[] = [
      el("div", { class: "row spread wrap", style: "align-items:flex-start" }, [
        el("div", { class: "v", text: `${a.icon} ${a.name}` }),
        el("div", { class: "row wrap", style: "gap:4px" }, titleChips),
      ]),
      el("div", { class: "muted", text: a.desc, style: "font-size:.88em" }),
      el("div", {}, [button("ℹ 事例と対策", () => openCaseModal(a), "btn sm ghost")]),
      ...slotRows,
      ...filterRows,
    ];
    if (!ready) {
      kids.push(
        el("div", {
          class: "mute2",
          text: missing.length
            ? `必要な情報が足りません（必要: ${missing.join(" / ")}）`
            : "いまは実行できません",
          style: "font-size:.85em",
        }),
      );
    }
    kids.push(button(`実行（所要 ${a.time_text}）`, run, "btn primary sm", !ready));

    return el(
      "div",
      {
        class: "fieldcard stack",
        dataset: { action: a.key },
        style: `border-left-color:${a.illegal ? "var(--danger)" : "var(--accent)"};${ready ? "" : "opacity:.6"}`,
      },
      kids,
    );
  }

  function openCaseModal(a: ActionInfo): void {
    openModal(
      `${a.icon} ${a.name}`,
      el("div", { class: "stack" }, [
        el("h4", { text: "実際の事例", style: "margin:0" }),
        el("div", { class: "note-case", text: a.case }),
        el("h4", { text: "対策", style: "margin:6px 0 0" }),
        el("div", { class: "note-counter", text: a.counter }),
      ]),
    );
  }

  /** 指定の手口のカードへ移動する（必要ならカテゴリを切り替え、入力欄を事前選択する） */
  function gotoAction(key: string, preset?: { slot: number; value: string }): void {
    const a = ctx.catalog.find((x) => x.key === key);
    if (!a || selected === null || actionsHost === null || currentView === null) return;
    activeCat = a.cat;
    if (preset) formFor(selected, key).slots[preset.slot] = preset.value;
    fillActions(actionsHost, currentView, selected);
    const card = actionsHost.querySelector<HTMLElement>(`[data-action="${key}"]`);
    if (!card) return;
    card.scrollIntoView({ behavior: "smooth", block: "center" });
    card.style.outline = "2px solid var(--accent)";
    setTimeout(() => {
      card.style.outline = "";
    }, 1800);
  }

  // ---- 取得情報・手がかり・ログ ----

  function factsPanel(tv: TargetView): HTMLElement {
    const card = el("div", { class: "card stack" });
    card.append(el("h3", { text: `📥 取得した情報（${tv.facts.length}件）`, style: "margin:0" }));
    if (tv.facts.length === 0) {
      card.append(el("div", { class: "empty", text: "まだ情報がありません。手口を実行して集めましょう。" }));
      return card;
    }
    card.append(
      el("div", { class: "muted", text: "カードを押すと、その情報にどうやってたどり着いたか（由来）が見られます。", style: "font-size:.82em" }),
    );
    const byGroup = new Map<string, FactView[]>();
    for (const f of tv.facts) {
      const list = byGroup.get(f.group);
      if (list) list.push(f);
      else byGroup.set(f.group, [f]);
    }
    const order = [...Object.keys(GROUP_LABEL), ...[...byGroup.keys()].filter((k) => !(k in GROUP_LABEL))];
    for (const g of order) {
      const list = byGroup.get(g);
      if (!list) continue;
      card.append(
        el("div", { class: "row", style: "margin-top:4px" }, [
          el("span", { class: "dot", style: `background:${GROUP_COLOR[g] ?? "var(--accent)"}` }),
          el("b", { text: GROUP_LABEL[g] ?? g }),
          chip(`${list.length}`, "sm"),
        ]),
        el("div", { class: "grid cols2" }, list.map(factCard)),
      );
    }
    return card;
  }

  function cluesPanel(tv: TargetView, target: number): HTMLElement {
    const card = el("div", { class: "card stack" });
    card.append(el("h3", { text: `📷 手がかり写真（${tv.clues.length}枚）`, style: "margin:0" }));
    if (tv.clues.length === 0) {
      card.append(
        el("div", {
          class: "empty",
          text: "まだ写真はありません。SNSの投稿を見て、写真つきの投稿を記録すると、ここに並びます。",
        }),
      );
      return card;
    }
    for (const c of tv.clues) card.append(clueCard(c, target));
    return card;
  }

  function clueCard(c: ClueView, target: number): HTMLElement {
    const head: (Node | string)[] = [
      el("div", { class: "row", style: "gap:6px" }, [
        chip(c.platform, "sm"),
        el("span", { class: "muted", text: c.date, style: "font-size:.8em" }),
      ]),
    ];
    if (c.correct === false) head.push(chip("✗ 別人の情報かも", "sm tag-illegal"));
    const kids: (Node | string)[] = [
      el("div", { class: "row spread" }, head),
      el("div", { text: c.text, style: "white-space:pre-wrap;overflow-wrap:anywhere" }),
      el("div", { class: "muted", text: `📷 ${c.caption}`, style: "font-size:.88em" }),
    ];

    const tags: (Node | string)[] = [];
    if (c.analyzed) {
      if (c.features.length === 0) tags.push(el("span", { class: "muted", text: "写り込みから分かるものはなかった", style: "font-size:.85em" }));
      for (const f of c.features) tags.push(chip(`🔍 ${f}`, "sm"));
      if (c.street_ok) tags.push(chip("🗺 街並み照合に使える", "sm", "color:var(--ok)"));
    } else {
      tags.push(
        button(
          `🔍 写り込みを調べる（${ctx.catalog.find((x) => x.key === "photo_reflect")?.time_text ?? ""}）`,
          () => exec({ action: "photo_reflect", target, clue: c.id }),
        ),
      );
    }
    if (c.exif_checked) tags.push(chip("🛰 Exif確認済み", "sm"));
    else tags.push(button(`🛰 ${actionLabel("photo_exif")}`, () => exec({ action: "photo_exif", target, clue: c.id })));
    tags.push(button("由来", () => openClueModal(c), "btn sm ghost"));
    kids.push(el("div", { class: "row wrap", style: "gap:6px" }, tags));

    return el("div", { class: "fieldcard stack", style: `border-left-color:${GROUP_COLOR.social}` }, kids);
  }

  function logPanel(tv: TargetView): HTMLElement {
    const card = el("div", { class: "card stack" });
    card.append(el("h3", { text: "📜 調査ログ", style: "margin:0" }));
    if (tv.log.length === 0) {
      card.append(el("div", { class: "empty", text: "まだ何も実行していません" }));
      return card;
    }
    const rows = [...tv.log].reverse().map((l) =>
      el("tr", {}, [
        el("td", { class: "num", text: l.date }),
        el("td", { text: l.action }),
        el("td", { text: l.result, style: "overflow-wrap:anywhere" }),
        el("td", { class: "num", text: l.cost_text }),
        el("td", { text: l.ok ? "✓" : "✗", style: `color:${l.ok ? "var(--ok)" : "var(--danger)"}` }),
      ]),
    );
    card.append(
      el("div", { class: "scroll" }, [
        el("table", { class: "tbl" }, [
          el("thead", {}, [
            el("tr", {}, [
              el("th", { text: "日付" }),
              el("th", { text: "手口" }),
              el("th", { text: "結果" }),
              el("th", { text: "所要" }),
              el("th", { text: "成否" }),
            ]),
          ]),
          el("tbody", {}, rows),
        ]),
      ]),
    );
    return card;
  }

  // ---- 結果エリア ----

  function resultCard(target: number, rs: ResultState): HTMLElement {
    const card = el("div", {
      class: "card stack",
      style: `border-left:3px solid ${rs.ok ? "var(--ok)" : "var(--danger)"};${SCROLL_MARGIN}`,
    });
    card.append(
      el("div", { class: "row spread" }, [
        el("h3", { text: "📋 実行結果", style: "margin:0" }),
        button("閉じる ×", () => {
          results.delete(target);
          renderDetail();
        }, "btn sm ghost"),
      ]),
      el("div", { text: `${rs.ok ? "✓" : "✗"} ${rs.message}`, style: `font-weight:600;color:${rs.ok ? "var(--ok)" : "var(--danger)"}` }),
    );
    if (rs.minutes > 0) card.append(el("div", { class: "muted", text: `⏱ ${rs.timeText}かかった`, style: "font-size:.88em" }));
    if (rs.detail.length) {
      card.append(el("ul", { style: "margin:0;padding-left:20px" }, rs.detail.map((d) => el("li", { text: d }))));
    }
    const added: (Node | string)[] = [
      ...rs.newFacts.map((f) => chip(`＋ ${f.label}: ${f.value}`, "sm", "color:var(--ok)")),
      ...rs.newClues.map((c) => chip(`＋ 写真: ${c.caption}`, "sm", "color:var(--ok)")),
    ];
    if (added.length) {
      card.append(
        el("div", { class: "stack", style: "gap:4px" }, [
          el("div", { class: "muted", text: "新しく記録した情報", style: "font-size:.8em" }),
          el("div", { class: "row wrap", style: "gap:4px" }, added),
        ]),
      );
    }
    if (rs.feed) card.append(feedNode(target, rs, rs.feed));
    if (rs.accounts) card.append(accountsNode(target, rs, rs.accounts));
    if (rs.hits) card.append(hitsNode(target, rs, rs.hits));
    return card;
  }

  function section(title: string): HTMLElement {
    return el("div", { class: "stack", style: "border-top:1px solid var(--line);padding-top:10px;margin-top:4px" }, [
      el("h4", { text: title, style: "margin:0" }),
    ]);
  }

  function accountHead(acc: AccountView): HTMLElement {
    const lock = acc.public ? "公開" : acc.accessible ? "🔒 鍵（閲覧可）" : "🔒 鍵";
    const line: (Node | string)[] = [
      chip(`${acc.icon} ${acc.platform}`),
      el("b", { class: "mono", text: `@${acc.handle}` }),
      el("span", { text: acc.display }),
      chip(lock, "sm", acc.public ? "" : "color:var(--warn)"),
    ];
    if (acc.note) line.push(chip(acc.note, "sm", "color:var(--accent)"));
    const kids: (Node | string)[] = [el("div", { class: "row wrap", style: "gap:6px" }, line)];
    if (acc.bio) kids.push(el("div", { class: "muted", text: acc.bio, style: "font-size:.88em;white-space:pre-wrap;overflow-wrap:anywhere" }));
    return el("div", { class: "stack", style: "gap:4px" }, kids);
  }

  // SNSビューア
  function feedNode(target: number, rs: ResultState, feed: FeedView): HTMLElement {
    const sec = section("📱 SNSビューア");
    sec.append(accountHead(feed.account));

    // プロフィールの記録
    const profDone = feed.profile_done || rs.done.has(`extract_profile:${feed.profile_token}`);
    if (profDone) sec.append(chip("✓ プロフィール記録済み", "sm", "color:var(--ok);align-self:flex-start"));
    else if (feed.profile_info) {
      sec.append(
        el("div", {}, [button("📝 プロフィールを記録", () => exec({ action: "extract_profile", target, token: feed.profile_token }))]),
      );
    } else sec.append(el("div", { class: "mute2", text: "プロフィールに手がかりはなさそうです。", style: "font-size:.85em" }));

    if (feed.locked) {
      sec.append(
        box("warn", "🔒 鍵アカウント：投稿は見られません（プロフィールのみ）。"),
        el("div", { class: "muted", style: "font-size:.88em", text: "「なりすましフォロー申請」の手口で承認されれば、投稿を読めるかもしれません。" }),
        el("div", {}, [
          button("🎭 なりすましフォロー申請へ", () => gotoAction("fake_follow", { slot: 0, value: optValue({ kind: "fact", id: feed.fact }) })),
        ]),
      );
      return sec;
    }

    const pager = (): HTMLElement =>
      el("div", { class: "row spread wrap" }, [
        button("◀ 前へ", () => exec({ action: "sns_view", target, fact: feed.fact, page: feed.page - 1 }), "btn sm", feed.page <= 0),
        el("span", { class: "muted", text: `${feed.page + 1} / ${feed.pages} ページ（全${feed.total}件）`, style: "font-size:.85em" }),
        button("次へ ▶", () => exec({ action: "sns_view", target, fact: feed.fact, page: feed.page + 1 }), "btn sm", feed.page + 1 >= feed.pages),
      ]);

    if (feed.posts.length === 0) {
      sec.append(el("div", { class: "empty", text: "まだ投稿がありません" }));
      return sec;
    }
    sec.append(
      pager(),
      el("div", { class: "mute2", text: `ページを読み進めるたびに所要時間（${actionLabel("sns_view")}）が進みます。`, style: "font-size:.78em" }),
    );
    for (const p of feed.posts) sec.append(postNode(target, rs, p));
    sec.append(pager());
    return sec;
  }

  function postNode(target: number, rs: ResultState, p: PostView): HTMLElement {
    const done = p.done || rs.done.has(`extract:${p.token}`);
    const head: (Node | string)[] = [el("span", { class: "muted", text: p.date, style: "font-size:.8em" })];
    if (p.info) head.push(chip("💡 手がかりあり", "sm", "color:var(--warn)"));
    const kids: (Node | string)[] = [
      el("div", { class: "row spread" }, head),
      el("div", { text: p.text, style: "white-space:pre-wrap;overflow-wrap:anywhere" }),
    ];
    if (p.photo) kids.push(el("div", { class: "muted", text: `📷 (${p.photo})`, style: "font-size:.88em" }));
    if (p.info) {
      kids.push(
        done
          ? chip("✓ 記録済み", "sm", "color:var(--ok);align-self:flex-start")
          : el("div", {}, [button("📥 この投稿から記録", () => exec({ action: "extract", target, token: p.token }))]),
      );
    }
    return el(
      "div",
      {
        class: "fieldcard stack",
        style: p.info ? "border-left-color:var(--warn);background:#2a1d12;gap:4px" : "border-left-color:var(--line);gap:4px",
      },
      kids,
    );
  }

  // 見つかったアカウント一覧
  function accountsNode(target: number, rs: ResultState, list: AccountView[]): HTMLElement {
    const sec = section(`🔎 見つかったアカウント（${list.length}件）`);
    if (rs.panelAction === "name_search") {
      sec.append(box("warn", "実名検索の結果には、同姓同名の別人が混ざります。プロフィール（居住地・学歴など）を見比べて見極めてください。"));
    } else if (rs.panelAction === "face_search") {
      sec.append(box("warn", "顔が似ているだけの別人が混ざることがあります。類似度だけで決めつけず、プロフィールを見比べてください。"));
    }
    if (list.length === 0) {
      sec.append(el("div", { class: "empty", text: "該当するアカウントは見つかりませんでした" }));
      return sec;
    }
    const labelOf = new Map(ctx.game.targets().map((s) => [s.id, s.label]));
    for (const acc of list) {
      const linked = acc.known === target || rs.done.has(`link_account:${acc.token}`);
      const profDone = rs.done.has(`extract_profile:${acc.token}`);
      const acts: (Node | string)[] = [];
      if (linked) acts.push(chip("✓ この対象のアカウントとして記録済み", "sm", "color:var(--ok)"));
      else acts.push(button("🔗 この対象のアカウントとして記録", () => exec({ action: "link_account", target, token: acc.token })));
      if (acc.known === null && !rs.done.has(`new_target_account:${acc.token}`)) {
        acts.push(button("👤 別人として新しい調査対象にする", () => exec({ action: "new_target_account", token: acc.token })));
      } else if (acc.known !== null && acc.known !== target) {
        const other = acc.known;
        acts.push(chip(`既に別の調査対象（${labelOf.get(other) ?? `#${other}`}）に紐付け済み`, "sm", "color:var(--warn)"));
        acts.push(button("その対象を開く", () => focus(other), "btn sm ghost"));
      }
      if (acc.bio) {
        if (profDone) acts.push(chip("✓ プロフィール記録済み", "sm", "color:var(--ok)"));
        else acts.push(button("📝 プロフィールを記録", () => exec({ action: "extract_profile", target, token: acc.token }), "btn sm ghost"));
      }
      sec.append(
        el("div", { class: "fieldcard stack", style: "border-left-color:var(--contact);gap:6px" }, [
          accountHead(acc),
          el("div", { class: "row wrap", style: "gap:6px" }, acts),
        ]),
      );
    }
    return sec;
  }

  // 流出データの名寄せ結果
  function hitsNode(target: number, rs: ResultState, h: HitsView): HTMLElement {
    const sec = section("🗃 流出データの名寄せ結果");
    const info: (Node | string)[] = [chip(`🔑 検索キー: ${h.key}`, "sm")];
    for (const f of h.filters) info.push(chip(f, "sm", "color:var(--accent)"));
    info.push(chip(`検索した流通データ ${h.searched}件`, "sm"), chip(`ヒット ${h.total}件`, "sm", h.total > 0 ? "color:var(--warn)" : ""));
    sec.append(el("div", { class: "row wrap", style: "gap:4px" }, info));
    if (h.note) sec.append(box("warn", h.note));
    sec.append(
      el("div", {
        class: "muted",
        text: "同じ人物のレコードだと判断できれば「同一人物として紐付け」、別人なら「別人として調査対象に追加」します。",
        style: "font-size:.82em",
      }),
    );
    if (h.groups.length === 0) {
      sec.append(el("div", { class: "empty", text: "流通している流出データにはヒットしませんでした" }));
      return sec;
    }
    for (const g of h.groups) {
      const inc = g.incident;
      const gbox = el("div", { class: "stack", style: "border:1px solid var(--line);border-radius:8px;padding:10px;gap:8px" }, [
        el("div", { class: "row wrap", style: "gap:6px" }, [
          el("b", { text: inc.company }),
          chip(inc.category, "sm"),
          chip(inc.date, "sm"),
          chip(inc.count_text, "sm"),
        ]),
      ]);
      if (g.namesakes) gbox.append(el("div", { class: "muted", text: `👥 ${g.namesakes}`, style: "font-size:.85em" }));
      if (g.hits.length === 0) gbox.append(el("div", { class: "mute2", text: "この事件のデータから直接ヒットしたレコードはありません。", style: "font-size:.85em" }));
      for (const hit of g.hits) gbox.append(hitNode(target, rs, hit));
      sec.append(gbox);
    }
    return sec;
  }

  function hitNode(target: number, rs: ResultState, hit: HitView): HTMLElement {
    const linked = hit.done || rs.done.has(`link_hit:${hit.token}`) || rs.done.has(`new_target_hit:${hit.token}`);
    const rows = hit.fields.map((kv) =>
      el("tr", {}, [
        el("td", { class: "muted", text: kv.label, style: "white-space:nowrap;width:1%" }),
        el("td", { text: kv.value, style: "overflow-wrap:anywhere" }),
      ]),
    );
    const acts: (Node | string)[] = linked
      ? [chip("✓ 記録済み", "sm", "color:var(--ok)")]
      : [
          button("🔗 同一人物として紐付け", () => exec({ action: "link_hit", target, token: hit.token })),
          button("👤 別人として調査対象に追加", () => exec({ action: "new_target_hit", token: hit.token })),
        ];
    return el("div", { class: "fieldcard stack", style: "border-left-color:var(--sensitive);gap:6px" }, [
      el("table", { class: "tbl" }, [el("tbody", {}, rows)]),
      el("div", { class: "row wrap", style: "gap:6px" }, acts),
    ]);
  }

  refresh();
  return { refresh, focus };
}
