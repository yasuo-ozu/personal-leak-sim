// ニュース / 流出事件 / 自分の流出 / 取った情報 / 地図 の各パネル。
// 登場する人物・企業・住所はすべて架空。Ctx 経由で Game を読み、refresh() のたびに描画し直す。

import L from "leaflet";
import type { EventView, FactView, Geo, IncidentView, SelfLeak, Summary } from "./api";
import type { Ctx, Panel } from "./store";
import { GROUP_COLOR, GROUP_LABEL, clear, el, geoColor, levelColor, riskColor, riskLabel } from "./store";

// ============================================================
// 共通ヘルパ
// ============================================================

const AGE_LABELS = ["0代", "10代", "20代", "30代", "40代", "50代", "60代", "70代", "80+"];
const LEVEL_TEXT = ["", "軽微", "注意", "重大", "深刻", "極めて深刻"];
const STYLE_DANGER = "background:#3a1518;color:#ff9ca0";
const STYLE_CIRC = "background:#2a1740;color:#d8b4fe";
const COLOR_FEMALE = "#ec6fb0";
const COLOR_MALE = "#4f8ef7";

/** 情報単体の危険度（Rust 側 Field::risk と同じ重み。snake_case のフィールド名で引く） */
const FIELD_RISK: Record<string, number> = {
  address: 25, town: 12, postal: 10, station: 8, city: 4, pref: 2, old_address: 8, routine: 10,
  name: 6, kana: 1, alias: 1, gender: 1, birth_date: 12, birth_md: 4, age: 2, face: 10, id_doc: 15,
  mobile: 10, home_phone: 6, email: 6, handle: 3,
  school: 8, workplace: 8, family: 6, hobby: 1, idol: 1, pet: 3,
  income: 4, assets: 8, card: 10, password: 15, history: 4, health: 10,
};

interface PillItem<T extends string> {
  key: T;
  label: string;
}

/** 切り替えピル。クリックで active を付け替えて onChange を呼ぶ */
function pillTabs<T extends string>(items: PillItem<T>[], current: T, onChange: (k: T) => void): HTMLElement {
  const wrap = el("div", { class: "pill-tabs" });
  const btns = new Map<T, HTMLElement>();
  for (const it of items) {
    const b = el("button", {
      class: it.key === current ? "pill active" : "pill",
      type: "button",
      text: it.label,
      onClick: () => {
        for (const [k, x] of btns) x.classList.toggle("active", k === it.key);
        onChange(it.key);
      },
    });
    btns.set(it.key, b);
    wrap.append(b);
  }
  return wrap;
}

function chip(text: string, style = "", small = false): HTMLElement {
  return el("span", { class: small ? "chip sm" : "chip", text, style });
}

function clampLevel(level: number): number {
  return Math.min(Math.max(Math.round(level), 1), 5);
}

/** 深刻度(1-5) を ●●●○○ で */
function levelDots(level: number): HTMLElement {
  const lv = clampLevel(level);
  const c = levelColor(lv);
  const wrap = el("span", { style: "display:inline-flex;gap:2px;align-items:center", title: `深刻度 ${lv}/5` });
  for (let i = 1; i <= 5; i++) {
    wrap.append(el("span", { class: "dot", style: `width:8px;height:8px;background:${i <= lv ? c : "#3a4256"}` }));
  }
  return wrap;
}

function levelBadge(level: number): HTMLElement {
  const lv = clampLevel(level);
  return el("span", { class: "row", style: "gap:6px;display:inline-flex" }, [
    levelDots(lv),
    el("span", {
      text: `深刻度 ${lv}（${LEVEL_TEXT[lv]}）`,
      style: `color:${levelColor(lv)};font-weight:600;font-size:.85em`,
    }),
  ]);
}

function kvCell(label: string, value: string | Node, accent = "var(--line)"): HTMLElement {
  return el("div", { class: "fieldcard", style: `border-left-color:${accent}` }, [
    el("div", { class: "mute2", text: label, style: "font-size:.75em" }),
    typeof value === "string"
      ? el("div", { class: "v", text: value, style: "overflow-wrap:anywhere" })
      : value,
  ]);
}

/** モーダルを開く。返り値は閉じる関数。背景クリック / Esc / ×で閉じる */
function openModal(title: string, body: Node): () => void {
  const bg = el("div", { class: "modal-bg" });
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") close();
  };
  const close = () => {
    document.removeEventListener("keydown", onKey);
    bg.remove();
  };
  const modal = el("div", { class: "modal", role: "dialog", "aria-modal": "true" }, [
    el("header", {}, [
      el("h3", { text: title, style: "margin:0" }),
      el("button", { class: "x", type: "button", "aria-label": "閉じる", text: "×", onClick: close }),
    ]),
    el("div", { class: "body" }, [body]),
  ]);
  bg.addEventListener("mousedown", (e) => {
    if (e.target === bg) close();
  });
  bg.append(modal);
  document.addEventListener("keydown", onKey);
  document.body.append(bg);
  return close;
}

/** 棒グラフ（.bars/.b/.lbl）。各棒の高さ = 値/最大*100%。ラベルの下に構成比(%)も出す */
function barChart(values: number[], labels: string[], color = "var(--accent-dim)", height = 90): HTMLElement {
  const max = Math.max(...values, 1e-9);
  const sum = values.reduce((a, b) => a + b, 0) || 1;
  const bars = el("div", { class: "bars", style: `height:${height}px;border-bottom:1px solid var(--line)` });
  const lbls = el("div", { class: "bars", style: "height:auto;align-items:flex-start;margin-top:3px" });
  values.forEach((v, i) => {
    const pct = (v / sum) * 100;
    bars.append(
      el("div", {
        class: "b",
        title: `${labels[i] ?? ""}: ${pct.toFixed(1)}%`,
        style: `height:${(v / max) * 100}%;background:${color}`,
      }),
    );
    lbls.append(
      el("div", { class: "lbl", style: "flex:1;min-width:0" }, [
        labels[i] ?? "",
        el("br"),
        el("span", { text: `${pct.toFixed(0)}%`, style: "color:var(--fg-dim)" }),
      ]),
    );
  });
  return el("div", {}, [bars, lbls]);
}

/** 男女比の横バー */
function genderBar(female: number): HTMLElement {
  const f = Math.round(Math.min(Math.max(female, 0), 1) * 100);
  const m = 100 - f;
  return el("div", {}, [
    el("div", { class: "row spread", style: "font-size:.85em;margin-bottom:4px" }, [
      el("span", { text: `女性 ${f}%`, style: `color:${COLOR_FEMALE};font-weight:600` }),
      el("span", { text: `男性 ${m}%`, style: `color:${COLOR_MALE};font-weight:600` }),
    ]),
    el("div", { style: "display:flex;height:12px;border-radius:999px;overflow:hidden;background:var(--bg3)" }, [
      el("span", { title: `女性 ${f}%`, style: `width:${f}%;background:${COLOR_FEMALE}` }),
      el("span", { title: `男性 ${m}%`, style: `width:${m}%;background:${COLOR_MALE}` }),
    ]),
  ]);
}

function sectionTitle(text: string): HTMLElement {
  return el("div", { text, style: "font-weight:600;margin-bottom:6px" });
}

/** 顧客層から読み取れる教育的な一言 */
function demographicsNote(iv: IncidentView): string {
  let top = 0;
  iv.age_dist.forEach((v, i) => {
    if (v > (iv.age_dist[top] ?? 0)) top = i;
  });
  const share = Math.round((iv.age_dist[top] ?? 0) * 100);
  const women = Math.round(iv.female * 100);
  const lean = women >= 60 ? "女性に偏っています" : women <= 40 ? "男性に偏っています" : "男女ほぼ半々です";
  return (
    `この事件の顧客層は ${AGE_LABELS[top] ?? ""} が最多（約${share}%）で、${lean}。` +
    "この年代・性別の構成から、あなたの知り合い（家族・同僚・同級生）の誰が含まれていそうか考えてみましょう。" +
    "本人が気づかないまま、データだけが出回っていることもあります。"
  );
}

/** 流出事件の詳細モーダル */
function incidentModal(iv: IncidentView): void {
  const body = el("div", { class: "stack", style: "gap:16px" });

  const badges = el("div", { class: "row wrap", style: "gap:6px" }, [levelBadge(iv.level), chip(iv.category), chip(`原因: ${iv.cause}`)]);
  if (iv.locality) badges.append(chip(`📍 地域限定: ${iv.locality}`));
  if (iv.idol) badges.append(chip(`🎤 ファン対象: ${iv.idol}`));
  if (iv.circulating) badges.append(chip("🕶 闇市場で流通中", STYLE_CIRC));
  if (iv.persona_hit) badges.append(chip("📧 あなたも対象", STYLE_DANGER));
  body.append(badges);

  if (iv.persona_hit) {
    body.append(
      el("div", {
        class: "note-case",
        text: "📧 あなたのデータもこの事件に含まれています。「自分の流出」タブで、何が漏れたのかを確認できます。",
      }),
    );
  }

  const grid = el("div", { class: "grid cols2" }, [
    kvCell("サービス", iv.service),
    kvCell("公表日", iv.announce),
    kvCell("発生時期", iv.occur),
    kvCell(
      "闇市場への流通",
      iv.circ ? `${iv.circ} ごろから出回り始めた` : "まだ確認されていません（今後出回る可能性はあります）",
      iv.circ ? "#a855f7" : "var(--line)",
    ),
    kvCell("原因", iv.cause),
    kvCell("件数", iv.count_text, levelColor(iv.level)),
    kvCell("規模", iv.pct_text, levelColor(iv.level)),
  ]);
  body.append(grid);

  const fieldBox = el("div", {}, [sectionTitle("流出した項目")]);
  const chips = el("div", { class: "row wrap", style: "gap:6px" });
  for (const f of iv.fields) chips.append(chip(f));
  if (iv.fields.length === 0) chips.append(el("span", { class: "muted", text: "（不明）" }));
  fieldBox.append(chips);
  body.append(fieldBox);

  body.append(
    el("div", {}, [
      sectionTitle("年代分布（流出した人の年代の構成比）"),
      barChart(iv.age_dist, AGE_LABELS),
    ]),
    el("div", {}, [sectionTitle("男女比"), genderBar(iv.female)]),
    el("div", { class: "note-case", text: `🧭 ${demographicsNote(iv)}` }),
    el("div", {
      class: "note-counter",
      text:
        "🛡 対策: 流出の通知が来たら、そのサービスのパスワードを変更し、同じパスワードを使い回しているサービスも変えましょう。" +
        "身に覚えのない電話・SMS・メールには、流出した情報を知っている相手でも応じないこと。",
    }),
  );

  openModal(`${iv.company} の流出事件`, body);
}

/** 画面が非表示（display:none）のとき true。重い描画をスキップするために使う */
function isHidden(root: HTMLElement): boolean {
  return root.getClientRects().length === 0;
}

/** 「もっと見る」ボタン */
function moreButton(remaining: number, step: number, onClick: () => void): HTMLElement {
  return el("div", { style: "text-align:center;margin-top:10px" }, [
    el("button", { class: "btn", type: "button", text: `もっと見る（残り ${remaining} 件、+${Math.min(step, remaining)} 件）`, onClick }),
  ]);
}

// ============================================================
// タイムライン
// ============================================================

type EventFilter = "all" | "mine" | "circ";

const EVENT_META: Record<EventView["kind"], { icon: string; label: string; style: string; title: string }> = {
  announce: { icon: "🗞", label: "公表", style: "", title: "" },
  notify: {
    icon: "📧",
    label: "あなた宛てのお詫び",
    style: "border-left-width:5px;background:rgba(229,72,77,.10);padding-right:10px;border-radius:0 8px 8px 0;margin-top:6px;margin-bottom:6px",
    title: "color:#ff9ca0",
  },
  circulate: {
    icon: "🕶",
    label: "闇市場で流通",
    style: "background:rgba(168,85,247,.08);padding-right:10px;border-radius:0 8px 8px 0;margin-top:4px;margin-bottom:4px",
    title: "color:#d8b4fe",
  },
};

function eventNode(e: EventView, ctx: Ctx): HTMLElement {
  const m = EVENT_META[e.kind];
  const head = el("div", { class: "row wrap", style: "gap:8px" }, [
    el("span", { class: "mute2 num", text: e.date, style: "font-size:.85em" }),
    chip(`${m.icon} ${m.label}`, e.kind === "notify" ? STYLE_DANGER : e.kind === "circulate" ? STYLE_CIRC : "", true),
  ]);
  if (e.incident !== null && e.level > 0) head.append(levelDots(e.level));

  const node = el("div", { class: `event ${e.kind}`, style: m.style }, [
    head,
    el("div", { text: e.title, style: `font-weight:600;margin-top:2px;${m.title}` }),
    el("div", { class: "muted", text: e.body, style: "white-space:pre-wrap;overflow-wrap:anywhere" }),
  ]);
  if (e.incident !== null) {
    const id = e.incident;
    node.append(
      el("div", { style: "margin-top:6px" }, [
        el("button", {
          class: "btn sm",
          type: "button",
          text: "詳細",
          onClick: () => {
            const iv = ctx.game.incident(id);
            if (iv) incidentModal(iv);
            else ctx.toast("事件の詳細を取得できませんでした", false);
          },
        }),
      ]),
    );
  }
  return node;
}

export function mountTimeline(root: HTMLElement, ctx: Ctx): Panel {
  const PAGE = 100;
  let filter: EventFilter = "all";
  let limit = PAGE;
  let showPast = false;
  let pastLimit = PAGE;

  const render = () => {
    const st = ctx.game.state();
    const startDay = st.day - st.elapsed_days;
    const all = ctx.game.events(0).slice().reverse(); // 新しい順
    const isPast = (e: EventView) => e.day < startDay;
    const match = (e: EventView) =>
      filter === "all" || (filter === "mine" ? e.kind === "notify" : e.kind === "circulate");
    const cur = all.filter((e) => !isPast(e) && match(e));
    const past = all.filter((e) => isPast(e) && match(e));

    const items: PillItem<EventFilter>[] = [
      { key: "all", label: `すべて (${all.length})` },
      { key: "mine", label: `自分の被害 (${all.filter((e) => e.kind === "notify").length})` },
      { key: "circ", label: `流通データ (${all.filter((e) => e.kind === "circulate").length})` },
    ];

    clear(root);
    root.append(
      el("div", { class: "card" }, [
        el("h2", { text: "🗞 ニュース" }),
        el("p", {
          class: "muted",
          text: "企業からの個人情報流出のニュースです。赤は「あなた宛てのお詫び」（自分のデータが含まれていた）、紫は闇市場でデータが出回り始めたという報せです。",
        }),
        pillTabs(items, filter, (k) => {
          filter = k;
          limit = PAGE;
          pastLimit = PAGE;
          render();
        }),
      ]),
    );

    const list = el("div", { class: "card", style: "margin-top:12px" });
    if (cur.length === 0) {
      list.append(
        el("div", {
          class: "empty",
          text:
            filter === "mine"
              ? "まだあなた宛てのお詫びは届いていません。時間を進めてみましょう。"
              : filter === "circ"
                ? "闇市場で流通中のデータはまだありません。"
                : "新しいニュースはまだありません。上の「時間を進める」を押してみましょう。",
        }),
      );
    } else {
      for (const e of cur.slice(0, limit)) list.append(eventNode(e, ctx));
      if (cur.length > limit) list.append(moreButton(cur.length - limit, PAGE, () => { limit += PAGE; render(); }));
    }
    root.append(list);

    if (past.length > 0 || all.some(isPast)) {
      const box = el("div", { class: "card", style: "margin-top:12px" });
      box.append(
        el("div", { class: "row spread" }, [
          el("div", {}, [
            el("b", { text: "📜 過去の事件" }),
            el("span", { class: "mute2", text: `（シミュレーション開始より前・${past.length}件）`, style: "margin-left:6px;font-size:.85em" }),
          ]),
          el("button", {
            class: "btn sm",
            type: "button",
            text: showPast ? "たたむ" : "ひらく",
            onClick: () => {
              showPast = !showPast;
              render();
            },
          }),
        ]),
      );
      if (showPast) {
        const inner = el("div", { style: "margin-top:8px" });
        if (past.length === 0) inner.append(el("div", { class: "empty", text: "該当する過去の事件はありません。" }));
        for (const e of past.slice(0, pastLimit)) inner.append(eventNode(e, ctx));
        if (past.length > pastLimit) inner.append(moreButton(past.length - pastLimit, PAGE, () => { pastLimit += PAGE; render(); }));
        box.append(inner);
      }
      root.append(box);
    }
  };

  return { refresh: render };
}

// ============================================================
// 流出事件の一覧
// ============================================================

type IncidentFilter = "all" | "circ" | "mine";

function incidentCard(iv: IncidentView): HTMLElement {
  const open = () => incidentModal(iv);
  const badges = el("div", { class: "row wrap", style: "gap:6px" });
  if (iv.persona_hit) badges.append(chip("📧 あなたも対象", STYLE_DANGER));
  if (iv.circulating) badges.append(chip("🕶 闇市場で流通中", STYLE_CIRC));
  if (iv.past) badges.append(chip("過去の事件", "", true));

  const sub = el("div", { class: "row wrap", style: "gap:6px;margin-top:6px" }, [chip(iv.category), chip(`原因: ${iv.cause}`)]);
  if (iv.locality) sub.append(chip(`📍 ${iv.locality}限定`));
  if (iv.idol) sub.append(chip(`🎤 ${iv.idol}ファン`));

  const meta = el("div", { class: "row wrap", style: "gap:4px 14px;margin-top:6px;font-size:.88em" }, [
    el("span", { class: "muted", text: `公表 ${iv.announce}` }),
    el("span", { text: iv.count_text, style: "font-weight:600" }),
    el("span", { class: "muted", text: iv.pct_text }),
  ]);

  const fields = el("div", { class: "row wrap", style: "gap:4px;margin-top:8px" });
  for (const f of iv.fields) fields.append(chip(f, "", true));

  return el(
    "div",
    {
      class: "card",
      role: "button",
      tabindex: "0",
      title: "クリックで詳細（顧客層の年代・男女比）を表示",
      style: `cursor:pointer;border-left:4px solid ${levelColor(iv.level)}`,
      onClick: open,
      onKeydown: (e: KeyboardEvent) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          open();
        }
      },
    },
    [
      el("div", { class: "row spread wrap" }, [
        el("div", { class: "row wrap", style: "gap:8px" }, [
          levelDots(iv.level),
          el("b", { text: iv.company }),
          iv.service && iv.service !== iv.company ? el("span", { class: "mute2", text: iv.service, style: "font-size:.85em" }) : "",
        ]),
        badges,
      ]),
      sub,
      meta,
      fields,
    ],
  );
}

export function mountIncidents(root: HTMLElement, ctx: Ctx): Panel {
  const PAGE = 60;
  let filter: IncidentFilter = "all";
  let limit = PAGE;

  const render = () => {
    const all = ctx.game
      .incidents()
      .slice()
      .sort((a, b) => b.announce_day - a.announce_day || b.id - a.id);
    const circ = all.filter((i) => i.circulating);
    const mine = all.filter((i) => i.persona_hit);
    const shown = filter === "all" ? all : filter === "circ" ? circ : mine;

    const items: PillItem<IncidentFilter>[] = [
      { key: "all", label: `すべて (${all.length})` },
      { key: "circ", label: `🕶 流通中 (${circ.length})` },
      { key: "mine", label: `📧 自分が対象 (${mine.length})` },
    ];

    clear(root);
    root.append(
      el("div", { class: "card" }, [
        el("h2", { text: "📊 流出事件" }),
        el("p", {
          class: "muted",
          text: "世の中で起きた個人情報の流出事件です。事件ごとに、流出した項目・規模・顧客層（年代／男女）が分かります。カードを押すと詳細が開きます。",
        }),
        pillTabs(items, filter, (k) => {
          filter = k;
          limit = PAGE;
          render();
        }),
      ]),
    );

    const list = el("div", { class: "stack", style: "margin-top:12px" });
    if (shown.length === 0) {
      list.append(
        el("div", {
          class: "card empty",
          text:
            filter === "mine"
              ? "あなたのデータを含む事件はまだありません。"
              : filter === "circ"
                ? "闇市場で流通中の事件はまだありません。時間を進めると増えていきます。"
                : "流出事件はまだありません。",
        }),
      );
    } else {
      for (const iv of shown.slice(0, limit)) list.append(incidentCard(iv));
      if (shown.length > limit) list.append(moreButton(shown.length - limit, PAGE, () => { limit += PAGE; render(); }));
    }
    root.append(list);
  };

  return { refresh: render };
}

// ============================================================
// 自分の流出状況
// ============================================================

function toggleSwitch(on: boolean, onChange: (v: boolean) => void): HTMLElement {
  const knob = el("span", {
    style: `position:absolute;top:2px;left:${on ? 22 : 2}px;width:20px;height:20px;border-radius:50%;background:#fff;transition:left .15s`,
  });
  return el(
    "button",
    {
      type: "button",
      role: "switch",
      "aria-checked": String(on),
      title: on ? "答え合わせ: ON" : "答え合わせ: OFF",
      style: `position:relative;flex:none;width:44px;height:24px;padding:0;border:0;border-radius:999px;background:${on ? "var(--accent)" : "#3a4256"};transition:background .15s`,
      onClick: () => onChange(!on),
    },
    [knob],
  );
}

/** 流出した項目の組み合わせから「名寄せされると何が起きるか」を作る */
function combineWarnings(exposed: string[]): string[] {
  const has = (...ls: string[]) => ls.some((l) => exposed.includes(l));
  const out: string[] = [];
  if (has("氏名") && has("住所")) out.push("氏名＋住所: 自宅への不審な郵送物、なりすまし請求、自宅を狙った訪問の足掛かりになります。");
  if (has("氏名") && has("携帯電話", "自宅電話")) out.push("氏名＋電話番号: 家族や取引先を装う詐欺電話・SMSが、本人にも知人にも精度高く届きます。");
  if (has("メールアドレス") && has("パスワード")) out.push("メール＋パスワード: 同じ組み合わせを使い回しているサービスへ次々と不正ログインされるおそれがあります。");
  if (has("氏名") && has("生年月日")) out.push("氏名＋生年月日: 本人確認の質問に答えられてしまい、契約変更やアカウント乗っ取りに悪用されえます。");
  if (has("クレジットカード")) out.push("カード情報: 不正利用のほか、「カード会社を名乗る確認電話」の信憑性を高めます。");
  if (has("顔写真", "身分証画像")) out.push("顔写真・身分証画像: なりすましでの口座開設や本人確認の突破に使われえます。");
  if (has("購入・利用履歴", "行動パターン")) out.push("利用履歴・行動パターン: 生活リズムや居場所の推測、狙いを絞った詐欺文面づくりに使われます。");
  if (has("健康・病歴", "年収", "資産状況")) out.push("健康・年収・資産: 弱みにつけ込む脅迫・詐欺や、差別的な扱いの材料になりえます。");
  if (out.length === 0 && exposed.length > 0) {
    out.push("今回の項目は単体では小さく見えても、別の流出データと突き合わされると、住所や連絡先までつながることがあります。");
  }
  return out;
}

function leakCard(leak: SelfLeak, ctx: Ctx): HTMLElement {
  const inc = leak.incident;
  const rows = leak.fields.map((kv) =>
    el("tr", {}, [
      el("td", { text: kv.label, style: "white-space:nowrap;color:var(--fg-dim);width:1%" }),
      el("td", { text: kv.value, style: "overflow-wrap:anywhere" }),
    ]),
  );
  return el("div", { class: "card", style: `border-left:4px solid ${leak.circulating ? "#a855f7" : "var(--danger)"}` }, [
    el("div", { class: "row spread wrap" }, [
      el("div", { class: "row wrap", style: "gap:8px" }, [
        el("b", { text: inc.company }),
        chip(inc.category),
        leak.circulating ? chip("🕶 闇市場で流通中", STYLE_CIRC) : "",
      ]),
      el("button", {
        class: "btn sm",
        type: "button",
        text: "事件の詳細",
        onClick: () => {
          const iv = ctx.game.incident(inc.id);
          if (iv) incidentModal(iv);
          else ctx.toast("事件の詳細を取得できませんでした", false);
        },
      }),
    ]),
    el("div", { class: "muted", text: `公表 ${inc.date} ・ ${inc.count_text}`, style: "font-size:.88em;margin:4px 0 8px" }),
    el("table", { class: "tbl" }, [
      el("thead", {}, [el("tr", {}, [el("th", { text: "項目" }), el("th", { text: "流出したあなたの情報" })])]),
      el("tbody", {}, rows),
    ]),
  ]);
}

export function mountSelf(root: HTMLElement, ctx: Ctx): Panel {
  const render = () => {
    const st = ctx.game.state();
    const me = ctx.game.selfExposure();
    clear(root);

    // 答え合わせトグル
    root.append(
      el("div", { class: "card" }, [
        el("div", { class: "row", style: "gap:12px" }, [
          toggleSwitch(st.reveal, (on) => {
            ctx.game.setReveal(on);
            ctx.refreshAll();
          }),
          el("div", { class: "grow" }, [
            el("div", { text: "答え合わせモード", style: "font-weight:600" }),
            el("div", {
              class: "muted",
              text: "ONにすると、調査画面で掴んだ情報が本当に本人のものか（正答）を確認できます。",
              style: "font-size:.88em",
            }),
          ]),
          chip(st.reveal ? "ON" : "OFF", st.reveal ? "background:var(--accent);color:#fff" : ""),
        ]),
      ]),
    );

    // 実際のプロフィール
    const prof = el("div", { class: "card", style: "margin-top:12px" }, [
      el("h2", { text: `${st.persona_icon} あなたの実際のプロフィール` }),
      el("p", {
        class: "muted",
        text: `「${st.persona_title}」としてのあなた（架空）の本当の情報です。流出すると、これらが他人の手に渡ります。`,
      }),
    ]);
    const pg = el("div", { class: "grid cols2" });
    for (const kv of me.profile) pg.append(kvCell(kv.label, kv.value, "var(--accent)"));
    prof.append(pg);
    if (me.accounts.length > 0) {
      const ag = el("div", { class: "grid cols2", style: "margin-top:10px" });
      for (const kv of me.accounts) ag.append(kvCell(kv.label, kv.value, "var(--social)"));
      prof.append(sectionTitle("SNSアカウント"), ag);
    }
    root.append(prof);

    // 露出度
    const rc = riskColor(me.risk);
    const exp = el("div", { class: "card", style: "margin-top:12px" }, [
      el("div", { class: "row spread" }, [
        el("h2", { text: "🫣 あなたの露出度", style: "margin:0" }),
        el("span", { text: `${me.risk} / 100（${riskLabel(me.risk)}）`, style: `color:${rc};font-weight:700` }),
      ]),
      el("div", { class: "meter", style: "margin:10px 0" }, [el("span", { style: `width:${Math.min(me.risk, 100)}%;background:${rc}` })]),
    ]);
    if (me.exposed.length > 0) {
      const chips = el("div", { class: "row wrap", style: "gap:6px" });
      for (const x of me.exposed) chips.append(chip(x, STYLE_DANGER));
      exp.append(el("div", { class: "muted", text: "これまでに流出した項目:", style: "font-size:.88em;margin-bottom:6px" }), chips);
    } else {
      exp.append(el("div", { class: "muted", text: "まだ流出した項目はありません。" }));
    }
    root.append(exp);

    // 巻き込まれた流出
    const leaks = el("div", { style: "margin-top:12px" });
    if (me.leaks.length === 0) {
      leaks.append(
        el("div", {
          class: "card empty",
          text: "まだ流出していません。時間を進めると、あなたのデータを含む流出事件が起きるかもしれません。",
        }),
      );
    } else {
      leaks.append(
        el("h3", { text: `あなたが巻き込まれた流出（${me.leaks.length}件）`, style: "margin:0 0 8px" }),
      );
      const warns = combineWarnings(me.exposed);
      const wbox = el("div", { class: "note-case" }, [
        el("div", { text: "⚠️ これらが名寄せされると何がわかるか", style: "font-weight:600;margin-bottom:4px" }),
        el("div", {
          class: "muted",
          text: "別々の事件で漏れた情報も、氏名・電話・メールなどをキーに「同じ人」としてまとめられると、1つの事件では分からなかった全体像が見えてしまいます。",
          style: "margin-bottom:6px",
        }),
      ]);
      const ul = el("ul", { style: "margin:0;padding-left:1.3em" });
      for (const w of warns) ul.append(el("li", { text: w }));
      wbox.append(ul);
      leaks.append(wbox);
      const stack = el("div", { class: "stack", style: "margin-top:10px" });
      for (const lk of me.leaks) stack.append(leakCard(lk, ctx));
      leaks.append(stack);
    }
    root.append(leaks);
  };

  return { refresh: render };
}

// ============================================================
// 取った情報（全ターゲット横断テーブル）
// ============================================================

type SortKey = "total" | "risk" | "target";

interface FactRow {
  s: Summary;
  f: FactView;
}

export function mountCollected(root: HTMLElement, ctx: Ctx): Panel {
  const PAGE = 200;
  let sort: SortKey = "total";
  let group = "all";
  let limit = PAGE;

  const linkBtn = (text: string, onClick: () => void, title?: string) =>
    el("button", {
      type: "button",
      text,
      title,
      style: "background:none;border:0;padding:0;color:var(--accent);text-align:left;text-decoration:underline;text-underline-offset:2px;font-weight:600",
      onClick,
    });

  const render = () => {
    if (isHidden(root)) return;
    const st = ctx.game.state();
    const reveal = st.reveal;

    // 全ターゲットの全 fact を集める
    const rows: FactRow[] = [];
    for (const s of ctx.game.targets()) {
      if (s.facts <= 0) continue;
      const tv = ctx.game.target(s.id);
      if (!tv) continue;
      for (const f of tv.facts) rows.push({ s: tv.summary, f });
    }
    const maxTotal = Math.max(1, ...rows.map((r) => r.f.total));

    clear(root);

    // ヘッダ + サマリー
    const stats = st.stats;
    const judged = rows.filter((r) => r.f.correct !== null);
    const sumItems: [string, string][] = [
      ["取得した情報", `${rows.length}件`],
      ["住所まで判明", `${stats.located}人 / ${stats.targets}人`],
      ["番地まで確定", `${stats.exact}人`],
      ["総調査時間", stats.spent_text],
    ];
    if (reveal && judged.length > 0) {
      sumItems.push(["正しい情報", `${judged.filter((r) => r.f.correct).length} / ${judged.length}件`]);
    }
    const summary = el("div", { class: "grid", style: "grid-template-columns:repeat(auto-fit,minmax(150px,1fr));margin-top:10px" });
    for (const [k, v] of sumItems) summary.append(kvCell(k, v, "var(--accent)"));

    const header = el("div", { class: "card" }, [
      el("h2", { text: "📁 取った情報" }),
      el("p", {
        class: "muted",
        text: "これまでに調査で集めた情報の一覧です。「累計時間」は、その情報に辿り着くまでの出どころ全体（SNS → 写真 → 住所 …）にかかった時間で、セルに重ねると経路が見えます。ふつうの人の情報が、短い時間でここまで集まってしまいます。",
      }),
      summary,
    ]);
    root.append(header);

    if (rows.length === 0) {
      root.append(el("div", { class: "card empty", style: "margin-top:12px", text: "まだ情報を集めていません。「調査」タブから始めましょう。" }));
      return;
    }

    // コントロール
    const sortItems: PillItem<SortKey>[] = [
      { key: "total", label: "累計時間が長い順" },
      { key: "risk", label: "危険度順" },
      { key: "target", label: "対象順" },
    ];
    const groupItems: PillItem<string>[] = [{ key: "all", label: `すべて (${rows.length})` }];
    for (const g of Object.keys(GROUP_LABEL)) {
      groupItems.push({ key: g, label: `${GROUP_LABEL[g]} (${rows.filter((r) => r.f.group === g).length})` });
    }
    const controls = el("div", { class: "card stack", style: "margin-top:12px" }, [
      el("div", { class: "row wrap", style: "gap:8px" }, [
        el("span", { class: "muted", text: "並び順:", style: "font-size:.85em" }),
        pillTabs(sortItems, sort, (k) => {
          sort = k;
          limit = PAGE;
          render();
        }),
      ]),
      el("div", { class: "row wrap", style: "gap:8px" }, [
        el("span", { class: "muted", text: "分類:", style: "font-size:.85em" }),
        pillTabs(groupItems, group, (k) => {
          group = k;
          limit = PAGE;
          render();
        }),
      ]),
    ]);
    root.append(controls);

    // フィルタ + ソート
    const shown = rows.filter((r) => group === "all" || r.f.group === group);
    const byTarget = (a: FactRow, b: FactRow) => a.s.id - b.s.id || a.f.id - b.f.id;
    shown.sort((a, b) => {
      if (sort === "total") return b.f.total - a.f.total || byTarget(a, b);
      if (sort === "risk") {
        return (FIELD_RISK[b.f.field] ?? 0) - (FIELD_RISK[a.f.field] ?? 0) || b.f.total - a.f.total || byTarget(a, b);
      }
      return byTarget(a, b);
    });

    // テーブル
    const heads = ["対象", "関係", "分類", "項目", "値", "入手方法", "かかった時間", "累計時間", "取得日"];
    if (reveal) heads.push("正誤");
    const thead = el("thead", {}, [el("tr", {}, heads.map((h) => el("th", { text: h, style: "white-space:nowrap" })))]);
    const tbody = el("tbody");
    for (const { s, f } of shown.slice(0, limit)) {
      const chain = f.chain.map((c) => `${c.label}: ${c.value}`).join(" → ");
      const targetCell = el("td", { style: "white-space:nowrap" }, [
        linkBtn(s.label, () => ctx.openTarget(s.id), "調査画面でこの対象を開く"),
      ]);
      if (s.geo && s.addr_level >= 2) {
        const geo = s.geo;
        targetCell.append(
          " ",
          el("button", {
            class: "btn sm",
            type: "button",
            text: "🗺 地図",
            title: `${s.addr_label}まで判明。地図で見る`,
            style: "margin-left:4px",
            onClick: () => ctx.flyTo(geo, s.label),
          }),
        );
      }
      const color = GROUP_COLOR[f.group] ?? "#8b8d98";
      const tr = el("tr", {}, [
        targetCell,
        el("td", { text: s.relation, style: "white-space:nowrap" }),
        el("td", { style: "white-space:nowrap" }, [
          el("span", { class: "dot", style: `background:${color};margin-right:5px` }),
          GROUP_LABEL[f.group] ?? f.group,
        ]),
        el("td", { text: f.label, style: "white-space:nowrap;font-weight:600" }),
        el("td", { text: f.value, style: "min-width:150px;overflow-wrap:anywhere" }),
        el("td", { text: f.via, style: "min-width:120px;color:var(--fg-dim)" }),
        el("td", { class: "num", text: f.cost_text }),
        el("td", { class: "num", title: chain ? `この情報に辿り着いた経路: ${chain}` : "", style: "min-width:84px" }, [
          el("b", { text: f.total_text }),
          el("div", { class: "meter", style: "height:3px;margin-top:3px" }, [
            el("span", { style: `width:${Math.max(2, (f.total / maxTotal) * 100)}%;background:var(--accent)` }),
          ]),
        ]),
        el("td", { class: "num", text: f.date }),
      ]);
      if (reveal) {
        tr.append(
          el("td", {
            class: "num",
            text: f.correct === null ? "—" : f.correct ? "✓" : "✗",
            title: f.correct === null ? "" : f.correct ? "正しい（この人物の情報）" : "誤り（別人の情報を掴んでいる）",
            style: `font-weight:700;color:${f.correct === null ? "var(--fg-mute)" : f.correct ? "var(--ok)" : "var(--danger)"}`,
          }),
        );
      }
      tbody.append(tr);
    }

    const tableBox = el("div", { class: "card", style: "margin-top:12px;padding:0" }, [
      el("div", { class: "scroll", style: "max-height:70vh;overflow:auto" }, [
        el("table", { class: "tbl", style: "min-width:1000px" }, [thead, tbody]),
      ]),
    ]);
    if (shown.length === 0) tableBox.append(el("div", { class: "empty", text: "この分類の情報はまだありません。" }));
    root.append(tableBox);
    if (shown.length > limit) root.append(moreButton(shown.length - limit, PAGE, () => { limit += PAGE; render(); }));
  };

  return { refresh: render };
}

// ============================================================
// 地図（Leaflet）
// ============================================================

function sameGeo(a: Geo, b: Geo): boolean {
  return Math.abs(a.lat - b.lat) < 1e-6 && Math.abs(a.lon - b.lon) < 1e-6;
}

function fmtRadius(r: number): string {
  return r >= 1000 ? `約${(r / 1000).toFixed(1)}km` : `約${Math.round(r)}m`;
}

/** 地図ポップアップ用の淡色チップ（ポップアップは白背景のため） */
function popChip(text: string): HTMLElement {
  return el("span", {
    text,
    style: "display:inline-block;padding:1px 7px;border-radius:999px;background:#eef0f5;color:#3b4150;font-size:.78em;white-space:nowrap",
  });
}

function popupLine(label: string, value: string | Node): HTMLElement {
  return el("div", { style: "margin-top:4px;font-size:.88em;color:#1f2430" }, [
    el("span", { text: `${label}: `, style: "color:#5b6070" }),
    value,
  ]);
}

export function mountMap(root: HTMLElement, ctx: Ctx): Panel & { flyTo(geo: Geo, label: string): void } {
  let map: L.Map | null = null;
  let targetLayer: L.FeatureGroup | null = null;
  let homeLayer: L.LayerGroup | null = null;
  let fitted = false;
  const entries: { label: string; geo: Geo; layer: L.Layer }[] = [];

  const status = el("div", { class: "muted", style: "font-size:.88em;margin-top:8px" });
  const legendDot = (color: string, text: string) =>
    el("span", { class: "row", style: "gap:5px;display:inline-flex;font-size:.82em" }, [
      el("span", { class: "dot", style: `background:${color}` }),
      el("span", { class: "muted", text }),
    ]);
  const legend = el("div", { class: "row wrap", style: "gap:4px 14px;margin-top:8px" }, [
    legendDot(geoColor(50), "〜50m（ほぼ自宅）"),
    legendDot(geoColor(500), "〜500m"),
    legendDot(geoColor(2000), "〜2km"),
    legendDot(geoColor(5000), "それ以上（市区町村レベル）"),
    el("span", { class: "muted", text: "📍 番地まで確定 ／ ● 推定 ／ ⭐ あなたの家", style: "font-size:.82em" }),
  ]);
  const head = el("div", { class: "card", style: "margin-bottom:12px" }, [
    el("h2", { text: "🗺 地図" }),
    el("div", {
      class: "note-case",
      text: "SNSの写真や流出データから、ここまで住所が絞り込まれてしまいます。丸の大きさは「推定の範囲」で、手がかりが増えるほど小さくなり、やがて自宅にたどり着きます。",
    }),
    status,
    legend,
  ]);
  const mapDiv = el("div", { id: "map", style: "position:relative;z-index:0" });
  root.append(head, mapDiv);

  const ensureMap = (): L.Map => {
    if (map) return map;
    const m = L.map(mapDiv, { center: [35.0, 136], zoom: 5 });
    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      maxZoom: 19,
      attribution: "© OpenStreetMap contributors",
    }).addTo(m);
    targetLayer = L.featureGroup().addTo(m);
    homeLayer = L.layerGroup().addTo(m);
    map = m;
    return m;
  };

  const popupNode = (s: Summary, geo: Geo): HTMLElement => {
    const node = el("div", { style: "min-width:210px;max-width:270px;color:#1f2430" }, [
      el("div", { class: "row wrap", style: "gap:6px" }, [
        el("b", { text: s.label, style: "font-size:1.02em" }),
        s.relation ? popChip(s.relation) : "",
      ]),
      popupLine("判明レベル", popChip(s.addr_label)),
      popupLine("住所", s.address ?? "（未判明）"),
      popupLine("位置の精度", geo.r > 0 ? `推定範囲 半径${fmtRadius(geo.r)}` : "ピンポイント"),
      popupLine(
        "危険度",
        el("b", { text: `${riskLabel(s.risk)}（${s.risk}）`, style: `color:${riskColor(s.risk)}` }),
      ),
    ]);
    if (s.addr_time_text) node.append(popupLine("⏱ 住所特定までの時間", el("b", { text: s.addr_time_text })));
    if (s.geo_wrong) {
      node.append(
        el("div", {
          text: "⚠️ 答え合わせ: この位置は別人の情報から割り出したもので、誤りです。",
          style: "margin-top:6px;font-size:.85em;color:#c62a30;font-weight:600",
        }),
      );
    }
    node.append(
      el("div", { style: "margin-top:8px" }, [
        el("button", {
          class: "btn sm primary",
          type: "button",
          text: "調査を続ける",
          onClick: () => {
            map?.closePopup();
            ctx.openTarget(s.id);
          },
        }),
      ]),
    );
    return node;
  };

  const refresh = () => {
    if (isHidden(root)) return; // 非表示のままだと地図サイズが 0 になる。表示されたときにもう一度呼ばれる
    const m = ensureMap();
    m.invalidateSize();
    if (!targetLayer || !homeLayer) return;
    targetLayer.clearLayers();
    homeLayer.clearLayers();
    entries.length = 0;

    const sums = ctx.game.targets();
    const plotted: { s: Summary; geo: Geo }[] = [];
    for (const s of sums) {
      if (s.geo && s.addr_level >= 2) plotted.push({ s, geo: s.geo });
    }
    // 広い円を先に描いて、狭い（濃い）ものを上に重ねる
    plotted.sort((a, b) => b.geo.r - a.geo.r);

    for (const { s, geo } of plotted) {
      if (geo.r > 50) {
        const color = geoColor(geo.r);
        L.circle([geo.lat, geo.lon], {
          radius: geo.r,
          color,
          weight: 1,
          fillColor: color,
          fillOpacity: 0.12,
        })
          .bindPopup(() => popupNode(s, geo))
          .addTo(targetLayer);
      }
    }
    for (const { s, geo } of plotted) {
      const color = geoColor(geo.r);
      let layer: L.Layer;
      if (s.addr_level >= 6) {
        layer = L.marker([geo.lat, geo.lon], {
          icon: L.divIcon({
            className: "leak-pin",
            html: el("div", { text: "📍", style: "font-size:24px;line-height:1;filter:drop-shadow(0 1px 2px rgba(0,0,0,.6))" }),
            iconSize: [24, 24],
            iconAnchor: [12, 22],
            popupAnchor: [0, -20],
          }),
          zIndexOffset: 500,
        });
      } else {
        layer = L.circleMarker([geo.lat, geo.lon], {
          radius: 7,
          color: "#ffffff",
          weight: 2,
          fillColor: color,
          fillOpacity: 1,
        });
      }
      layer.bindPopup(() => popupNode(s, geo)).addTo(targetLayer);
      entries.push({ label: s.label, geo, layer });
    }

    // 自分の家
    const home = ctx.game.state().home;
    L.marker([home.lat, home.lon], {
      icon: L.divIcon({
        className: "leak-home",
        html: el("div", {
          text: "⭐",
          style:
            "width:28px;height:28px;border-radius:50%;background:#2563eb;border:2px solid #fff;display:flex;align-items:center;justify-content:center;font-size:15px;box-shadow:0 1px 6px rgba(0,0,0,.5)",
        }),
        iconSize: [28, 28],
        iconAnchor: [14, 14],
        popupAnchor: [0, -14],
      }),
      zIndexOffset: 1000,
    })
      .bindPopup(
        () =>
          el("div", { style: "color:#1f2430" }, [
            el("b", { text: "⭐ あなたの家" }),
            el("div", { text: ctx.game.state().persona_name, style: "font-size:.88em;color:#5b6070" }),
          ]),
      )
      .addTo(homeLayer);

    // 初期表示: 判明した対象が収まるように。対象が無ければ日本全体
    if (!fitted && plotted.length > 0) {
      const b = targetLayer.getBounds().extend([home.lat, home.lon]);
      m.fitBounds(b.pad(0.25), { maxZoom: 14, animate: false });
      fitted = true;
    }

    const missing = sums.length - plotted.length;
    status.textContent =
      plotted.length > 0
        ? `地図に表示: ${plotted.length}人（市区町村レベル以上で判明）／ まだ場所が分からない対象: ${missing}人`
        : "まだ住所の手がかりがありません。調査で住所や最寄り駅などを掴むと、ここに表示されます。";
  };

  const flyTo = (geo: Geo, label: string) => {
    refresh();
    if (isHidden(root)) return;
    const m = ensureMap();
    m.invalidateSize();
    const target = L.latLng(geo.lat, geo.lon);
    const ent =
      entries.find((e) => e.label === label && sameGeo(e.geo, geo)) ??
      entries.find((e) => e.label === label) ??
      entries.find((e) => sameGeo(e.geo, geo));
    let opened = false;
    const open = () => {
      if (opened) return;
      opened = true;
      if (ent) ent.layer.openPopup();
      else L.popup().setLatLng(target).setContent(label).openOn(m);
    };
    m.once("moveend", open);
    window.setTimeout(open, 1800);
    if (geo.r > 300) m.flyToBounds(target.toBounds(geo.r * 2.4), { maxZoom: 16 });
    else m.flyTo(target, 16);
  };

  return { refresh, flyTo };
}
