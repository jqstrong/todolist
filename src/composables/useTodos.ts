/**
 * 待办的状态与动作。
 *
 * 顺序模型：tasks 数组的顺序就是手工顺序；显示时未完成在前、已完成沉底，
 * 两组各自保持数组内的相对顺序。因此“完成一件事”同时意味着“它沉到下面去”，
 * 这是一次动作、一个动画，而不是两次。
 */
import { computed, reactive, ref, watch } from "vue";
import type { Settings, Task } from "../types";
import { DATA_VERSION, defaultSettings, newTask } from "../types";
import { bridge } from "../bridge";

export interface StatusMessage {
  text: string;
  undo?: () => void;
}

interface UndoEntry {
  text: string;
  restore: () => void;
}

export function useTodos() {
  const tasks = ref<Task[]>([]);
  const settings = ref<Settings>(defaultSettings());
  const ready = ref(false);
  const message = ref<StatusMessage | null>(null);

  /** 刚被划掉、但还想让用户看见那道笔迹的条目 */
  const justCompleted = reactive(new Set<string>());
  const undoStack: UndoEntry[] = [];
  let messageTimer: number | undefined;

  /* ---------------------------------------------------------- 持久化 */

  let saveTimer: number | undefined;
  let saving = false;

  async function flush() {
    if (!ready.value || saving) return;
    saving = true;
    try {
      await bridge.save({ version: DATA_VERSION, tasks: tasks.value, settings: settings.value });
    } catch (err) {
      say(`保存失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      saving = false;
    }
  }

  function scheduleSave() {
    if (!ready.value) return;
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(flush, 250);
  }

  watch([tasks, settings], scheduleSave, { deep: true });

  async function load() {
    const data = await bridge.load();
    tasks.value = data.tasks;
    settings.value = data.settings;
    ready.value = true;
  }

  /* ---------------------------------------------------------- 状态行 */

  function say(text: string, undo?: () => void) {
    message.value = { text, undo };
    window.clearTimeout(messageTimer);
    messageTimer = window.setTimeout(() => (message.value = null), 6000);
  }

  function dismissMessage() {
    message.value = null;
    window.clearTimeout(messageTimer);
  }

  /* ---------------------------------------------------------- 读取 */

  const openCount = computed(() => tasks.value.filter((t) => !t.done).length);
  const doneCount = computed(() => tasks.value.filter((t) => t.done).length);

  /** 列表里真正显示的顺序 */
  const visibleTasks = computed<Task[]>(() => {
    const open = tasks.value.filter((t) => !t.done);
    const done = tasks.value.filter((t) => t.done && (settings.value.showCompleted || justCompleted.has(t.id)));
    return [...open, ...done];
  });

  /* ---------------------------------------------------------- 动作 */

  function add(title: string) {
    const text = title.trim();
    if (!text) return;
    // 新的写在最上面：回车之后不用滚动就能看到自己刚记下的东西
    tasks.value.unshift(newTask(text));
  }

  function find(id: string) {
    return tasks.value.find((t) => t.id === id);
  }

  function toggle(id: string) {
    const task = find(id);
    if (!task) return;
    task.done = !task.done;
    task.doneAt = task.done ? Date.now() : null;
    if (task.done) {
      // 若用户选择不显示已完成，也要留够时间把笔迹划完
      justCompleted.add(id);
      window.setTimeout(() => justCompleted.delete(id), 420);
    }
  }

  function rename(id: string, title: string) {
    const task = find(id);
    const text = title.trim();
    if (!task) return;
    if (!text) {
      remove(id);
      return;
    }
    task.title = text;
  }

  function remove(id: string) {
    const index = tasks.value.findIndex((t) => t.id === id);
    if (index < 0) return;
    const [task] = tasks.value.splice(index, 1);
    undoStack.push({
      text: `已删除「${clip(task.title)}」`,
      restore: () => tasks.value.splice(Math.min(index, tasks.value.length), 0, task),
    });
    say(`已删除「${clip(task.title)}」`, undo);
  }

  function clearDone() {
    const removed = tasks.value
      .map((task, index) => ({ task, index }))
      .filter(({ task }) => task.done);
    if (!removed.length) return;
    tasks.value = tasks.value.filter((t) => !t.done);
    undoStack.push({
      text: `已清除 ${removed.length} 项`,
      restore: () => {
        for (const { task, index } of removed) {
          tasks.value.splice(Math.min(index, tasks.value.length), 0, task);
        }
      },
    });
    say(`已清除 ${removed.length} 项已完成`, undo);
  }

  /** 在自己所属的组（未完成 / 已完成）内上下移动 */
  function move(id: string, direction: -1 | 1) {
    const task = find(id);
    if (!task) return;
    const group = tasks.value.filter((t) => t.done === task.done);
    const at = group.indexOf(task);
    const target = group[at + direction];
    if (!target) return;
    const a = tasks.value.indexOf(task);
    const b = tasks.value.indexOf(target);
    tasks.value[a] = target;
    tasks.value[b] = task;
  }

  function undo() {
    const entry = undoStack.pop();
    if (!entry) return;
    entry.restore();
    say(`已撤销：${entry.text}`);
  }

  function clip(text: string, max = 12) {
    return text.length > max ? `${text.slice(0, max)}…` : text;
  }

  return {
    tasks,
    settings,
    ready,
    message,
    openCount,
    doneCount,
    visibleTasks,
    load,
    flush,
    add,
    toggle,
    rename,
    remove,
    clearDone,
    move,
    undo,
    say,
    dismissMessage,
  };
}
