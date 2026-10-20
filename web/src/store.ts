// 画面間で共有する文脈（Ctx）と小さなヘルパ。
// 各パネルは mount(root, ctx) で描画し、refresh() を返す。Ctx 経由でのみ連携する。

import type { ActionInfo, Game, Geo, Resp } from "./api";

export interface Panel {
  refresh(): void;
}

export interface Ctx {
  game: Game;
  catalog: ActionInfo[];
  /** 調査タブを開き、対象にフォーカスする */
  openTarget(id: number): void;
  /** 地図タブを開いて該当地点へ移動 */
  flyTo(geo: Geo, label: string): void;
  /** 全パネルを再描画（時間経過・情報追加のあと） */
  refreshAll(): void;
  /** 画面下部に通知 */
  toast(msg: string, ok?: boolean): void;
  /** アクション結果に含まれる新着イベントを通知として流す */
  pushEvents(resp: Resp): void;
}

// ---- 表示ヘルパ ----

export const GROUP_LABEL: Record<string, string> = {
  identity: "本人確認に使える情報",
  location: "居場所に関する情報",
  contact: "連絡先",
  social: "人間関係・属性",
  sensitive: "特に機微な情報",
};

export const GROUP_COLOR: Record<string, string> = {
  identity: "#7c5cff",
  location: "#e5484d",
  contact: "#0ea5e9",
  social: "#12a594",
  sensitive: "#f76808",
};

/** 危険度(0-100) を色に */
export function riskColor(r: number): string {
  if (r >= 70) return "#e5484d";
  if (r >= 40) return "#f76808";
  if (r >= 15) return "#f5d90a";
  return "#30a46c";
}

export function riskLabel(r: number): string {
  if (r >= 70) return "危険";
  if (r >= 40) return "高";
  if (r >= 15) return "中";
  if (r > 0) return "低";
  return "—";
}

/** インシデントの深刻度(1-5) を色に */
export function levelColor(level: number): string {
  return ["#8b8d98", "#30a46c", "#f5d90a", "#f76808", "#e5484d", "#e5484d"][Math.min(level, 5)];
}

const ADDR_LABELS = ["不明", "都道府県", "市区町村", "最寄り駅", "町名／郵便番号", "丁目", "番地"];
export function addrLevelLabel(lv: number): string {
  return ADDR_LABELS[Math.min(lv, 6)] ?? "不明";
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: { class?: string; text?: string; html?: string; [k: string]: unknown } = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") node.className = String(v);
    else if (k === "text") node.textContent = String(v);
    else if (k === "html") node.innerHTML = String(v);
    else if (k === "dataset") Object.assign(node.dataset, v as object);
    else if (k.startsWith("on") && typeof v === "function") {
      node.addEventListener(k.slice(2).toLowerCase(), v as EventListener);
    } else node.setAttribute(k, String(v));
  }
  for (const c of children) node.append(c);
  return node;
}

export function clear(node: HTMLElement): HTMLElement {
  node.replaceChildren();
  return node;
}

export function esc(s: string): string {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

/** 緯度経度のあるフィールドのうち最も狭い範囲のものを住所推定として使う色 */
export function geoColor(r: number): string {
  if (r <= 50) return "#e5484d";
  if (r <= 500) return "#f76808";
  if (r <= 2000) return "#f5d90a";
  return "#0ea5e9";
}
