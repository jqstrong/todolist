/**
 * 前端与 Tauri 之间唯一的通道。
 *
 * 桌面端走 Rust 命令与事件；在纯浏览器里打开（设计走查、视觉回归）时自动降级为
 * localStorage + 空操作，因此同一份界面代码可以脱离桌面外壳单独预览。
 */
import type { AppData } from "./types";
import { defaultData, newTask, normalize } from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const LS_KEY = "gaozhi.data";

async function core() {
  return import("@tauri-apps/api/core");
}

/* ------------------------------------------------------------------ */
/* 浏览器预览用的样例数据（只在非 Tauri 环境、且本地无数据时使用）      */
/* ------------------------------------------------------------------ */

function demoData(kind: string | null): AppData {
  const data = defaultData();
  if (kind === "empty") return data;
  if (kind === "hideDone") data.settings.showCompleted = false;

  const titles =
    kind === "long"
      ? [
          "把季度复盘的提纲写出来，重点是这三个季度里哪些判断被证伪了，以及哪些只是运气好",
          "给服务器换证书",
          "读《技术的本质》第三章",
          "约牙医",
          "把上周的会议记录整理成三条结论发出去",
          "买咖啡豆",
          "重写 onboarding 文案的第二段，现在这段还是在解释功能而不是说人能得到什么",
          "交电费",
          "清理下载文件夹",
          "给妈妈打电话",
          "把显示器支架装好",
        ]
      : ["写设计说明", "修一个登录的 bug", "读《技术的本质》第三章", "买牛奶"];

  data.tasks = titles.map((title, i) => ({
    ...newTask(title),
    createdAt: Date.now() - i * 60000,
  }));

  // 一条已完成，用来看完成后的样子
  if (kind !== "long") {
    data.tasks[3].done = true;
    data.tasks[3].doneAt = Date.now() - 30000;
  } else {
    data.tasks[9].done = true;
    data.tasks[9].doneAt = Date.now() - 30000;
    data.tasks[10].done = true;
    data.tasks[10].doneAt = Date.now() - 20000;
  }
  return data;
}

/* ------------------------------------------------------------------ */

export const bridge = {
  isTauri,

  async load(): Promise<AppData> {
    if (!isTauri) {
      // 设计走查用的开关：?demo=empty|long  ?theme=night|paper
      const params = new URLSearchParams(location.search);
      const theme = params.get("theme");
      const fix = (data: AppData) => {
        if (theme === "night" || theme === "paper") data.settings.theme = theme;
        return data;
      };
      const raw = localStorage.getItem(LS_KEY);
      if (raw) {
        try {
          return fix(normalize(JSON.parse(raw)));
        } catch {
          /* 坏数据就重来 */
        }
      }
      return fix(demoData(params.get("demo")));
    }
    const { invoke } = await core();
    return normalize(await invoke("load_data"));
  },

  async save(data: AppData): Promise<void> {
    if (!isTauri) {
      localStorage.setItem(LS_KEY, JSON.stringify(data));
      return;
    }
    const { invoke } = await core();
    await invoke("save_data", { data });
  },

  async hideWindow(): Promise<void> {
    if (!isTauri) return;
    const { invoke } = await core();
    await invoke("hide_window");
  },

  async setAlwaysOnTop(value: boolean): Promise<void> {
    if (!isTauri) return;
    const { invoke } = await core();
    await invoke("set_always_on_top", { value });
  },

  /** 返回生效后的状态；失败时抛错，错误信息由 Rust 给出（中文）。 */
  async setAutostart(value: boolean): Promise<boolean> {
    if (!isTauri) return value;
    const { invoke } = await core();
    return await invoke<boolean>("set_autostart", { value });
  },

  async openDataDir(): Promise<void> {
    if (!isTauri) return;
    const { invoke } = await core();
    await invoke("open_data_dir");
  },

  async setHotkey(accelerator: string): Promise<void> {
    if (!isTauri) return;
    const { invoke } = await core();
    await invoke("set_hotkey", { accelerator });
  },

  async quit(): Promise<void> {
    if (!isTauri) return;
    const { invoke } = await core();
    await invoke("quit_app");
  },

  /** 窗口被召唤出来（快捷键 / 托盘）。返回取消订阅函数。 */
  async onShown(cb: () => void): Promise<() => void> {
    if (!isTauri) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return await listen("window-shown", () => cb());
  },

  /** Rust 端需要告诉用户的话（快捷键注册失败等）。 */
  async onNotice(cb: (message: string) => void): Promise<() => void> {
    if (!isTauri) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<string>("notice", (e) => cb(String(e.payload)));
  },

  /**
   * 取走启动期间留下的提示。
   * 注册全局快捷键发生在页面加载之前，那时广播的事件没有听众，
   * 所以要在挂载后主动取一次，否则「快捷键没能注册」这件事永远不会被说出来。
   */
  async takeNotice(): Promise<string | null> {
    if (!isTauri) return null;
    const { invoke } = await core();
    return (await invoke<string | null>("take_notice")) ?? null;
  },
};
