/**
 * 与 Rust 端一一对应的数据契约。
 * Rust 侧使用 #[serde(rename_all = "camelCase")]，字段名必须保持一致。
 * 契约文档见 docs/design-notes.md 与 src-tauri/src/store.rs。
 */

export interface Task {
  id: string;
  title: string;
  done: boolean;
  /** epoch 毫秒 */
  createdAt: number;
  /** epoch 毫秒；未完成时为 null */
  doneAt: number | null;
}

export type ThemeName = "system" | "paper" | "night";

export interface Settings {
  /** Tauri accelerator 语法，例如 "Ctrl+Alt+Space" */
  hotkey: string;
  hideOnBlur: boolean;
  alwaysOnTop: boolean;
  launchAtStartup: boolean;
  theme: ThemeName;
  showCompleted: boolean;
}

export interface AppData {
  version: number;
  tasks: Task[];
  settings: Settings;
}

export const DATA_VERSION = 1;

export function defaultSettings(): Settings {
  return {
    hotkey: "Ctrl+Alt+Space",
    hideOnBlur: true,
    alwaysOnTop: true,
    launchAtStartup: false,
    theme: "system",
    showCompleted: true,
  };
}

export function defaultData(): AppData {
  return { version: DATA_VERSION, tasks: [], settings: defaultSettings() };
}

export function newTask(title: string): Task {
  return {
    id:
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `t${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`,
    title,
    done: false,
    createdAt: Date.now(),
    doneAt: null,
  };
}

/** 把磁盘上读到的任意形状收敛成合法数据，坏字段一律回落到默认值。 */
export function normalize(raw: unknown): AppData {
  const base = defaultData();
  if (!raw || typeof raw !== "object") return base;
  const r = raw as Partial<AppData>;

  const settings = { ...base.settings, ...(r.settings ?? {}) };
  if (!["system", "paper", "night"].includes(settings.theme)) settings.theme = "system";
  if (typeof settings.hotkey !== "string" || !settings.hotkey.trim()) {
    settings.hotkey = base.settings.hotkey;
  }

  const tasks: Task[] = Array.isArray(r.tasks)
    ? r.tasks
        .filter((t): t is Task => !!t && typeof t === "object" && typeof (t as Task).title === "string")
        .map((t) => ({
          id: typeof t.id === "string" && t.id ? t.id : newTask("").id,
          title: String(t.title),
          done: !!t.done,
          createdAt: Number.isFinite(t.createdAt) ? t.createdAt : Date.now(),
          doneAt: Number.isFinite(t.doneAt as number) ? (t.doneAt as number) : null,
        }))
    : [];

  return { version: DATA_VERSION, tasks, settings };
}
