<script setup lang="ts">
/**
 * 面板本体：把状态、键盘模型和各部件接在一起。
 *
 * 键盘模型（面板被召唤出来时光标已经在召唤行里）：
 *   回车 记下 · ↑↓ 选择 · 空格 完成 · E 编辑 · DEL 删除 · ALT+↑↓ 排序 · CTRL+Z 撤销 · ESC 收起
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AppHead from "./components/AppHead.vue";
import CaptureRow from "./components/CaptureRow.vue";
import HintBar from "./components/HintBar.vue";
import SettingsSheet from "./components/SettingsSheet.vue";
import TaskRow from "./components/TaskRow.vue";
import { bridge } from "./bridge";
import { useTodos } from "./composables/useTodos";
import type { Settings, Task } from "./types";

const todos = useTodos();
const { settings, tasks, ready, message, openCount, visibleTasks } = todos;

/** 列表里的一行：要么是一条待办，要么是「已完成」那个分组的标签 */
type DisplayRow = { kind: "task"; task: Task } | { kind: "label"; count: number };

const displayRows = computed<DisplayRow[]>(() => {
  const open = visibleTasks.value.filter((t) => !t.done);
  const done = visibleTasks.value.filter((t) => t.done);
  const rows: DisplayRow[] = open.map((task) => ({ kind: "task", task }));
  if (done.length) {
    rows.push({ kind: "label", count: done.length });
    rows.push(...done.map((task): DisplayRow => ({ kind: "task", task })));
  }
  return rows;
});

const selectedId = ref<string | null>(null);
const editingId = ref<string | null>(null);
const settingsOpen = ref(false);
const scrolled = ref(false);
const atBottom = ref(false);
const overflowing = ref(false);
const today = ref("");
const hotkeyError = ref<string | null>(null);

const panel = ref<HTMLElement | null>(null);
const list = ref<HTMLElement | null>(null);
const capture = ref<InstanceType<typeof CaptureRow> | null>(null);

let stopShown: (() => void) | undefined;
let stopNotice: (() => void) | undefined;
const dark = window.matchMedia("(prefers-color-scheme: dark)");

/* ------------------------------------------------------------ 外观与召唤 */

function formatToday() {
  const d = new Date();
  return `${d.getMonth() + 1}月${d.getDate()}日 周${"日一二三四五六"[d.getDay()]}`;
}

function applyTheme() {
  const mode = settings.value.theme;
  const night = mode === "night" || (mode === "system" && dark.matches);
  document.documentElement.dataset.theme = night ? "night" : "paper";
}

/** 每次被召唤，面板重新“落”一次。这是全应用第一处、也是唯一一处入场动效。 */
function summon() {
  const el = panel.value;
  if (!el) return;
  el.classList.remove("is-summoning");
  void el.offsetWidth;
  el.classList.add("is-summoning");
}

async function onShown() {
  today.value = formatToday();
  selectedId.value = null;
  editingId.value = null;
  settingsOpen.value = false;
  await nextTick();
  summon();
  list.value?.scrollTo({ top: 0 });
  updateScroll();
  capture.value?.focus();
}

async function hide() {
  todos.dismissMessage();
  await todos.flush();
  await bridge.hideWindow();
}

/* ------------------------------------------------------------ 选择 */

function focusCapture() {
  selectedId.value = null;
  capture.value?.focus();
}

function rowElements() {
  return list.value ? Array.from(list.value.querySelectorAll<HTMLElement>(".row")) : [];
}

function reveal(index: number) {
  void nextTick(() => rowElements()[index]?.scrollIntoView({ block: "nearest" }));
}

function select(id: string) {
  selectedId.value = id;
  editingId.value = null;
  // 选中的是列表里的某一行，光标就该离开召唤行：
  // 否则空格会被当成打字，而不是“完成”。
  capture.value?.blur();
  reveal(visibleTasks.value.findIndex((t) => t.id === id));
}

/** ↑↓ 在列表里走；走到顶上就回到召唤行 */
function step(delta: -1 | 1) {
  const items = visibleTasks.value;
  if (!items.length) {
    focusCapture();
    return;
  }
  const at = items.findIndex((t) => t.id === selectedId.value);
  if (at < 0) {
    if (delta === 1) select(items[0].id);
    else focusCapture();
    return;
  }
  const next = at + delta;
  if (next < 0) {
    focusCapture();
    return;
  }
  if (next >= items.length) return;
  select(items[next].id);
}

/* ------------------------------------------------------------ 动作 */

function addTask(title: string) {
  todos.add(title);
  selectedId.value = null;
  void nextTick(() => {
    list.value?.scrollTo({ top: 0 });
    updateScroll();
    capture.value?.focus();
  });
}

function removeTask(id: string) {
  const at = visibleTasks.value.findIndex((t) => t.id === id);
  todos.remove(id);
  void nextTick(() => {
    const rest = visibleTasks.value;
    if (!rest.length) focusCapture();
    else selectedId.value = rest[Math.min(at, rest.length - 1)].id;
  });
}

function startEdit(id: string) {
  selectedId.value = id;
  editingId.value = id;
}

function commitEdit(id: string, title: string) {
  if (editingId.value !== id) return; // 回车提交后失焦可能再来一次，忽略
  editingId.value = null;
  todos.rename(id, title);
  void nextTick(() => capture.value?.focus());
}

function cancelEdit() {
  editingId.value = null;
  void nextTick(() => capture.value?.focus());
}

function onScroll() {
  updateScroll();
}

function updateScroll() {
  const el = list.value;
  if (!el) return;
  const overflow = el.scrollHeight - el.clientHeight;
  overflowing.value = overflow > 4;
  scrolled.value = el.scrollTop > 2;
  atBottom.value = overflow - el.scrollTop < 4;
}

/* ------------------------------------------------------------ 设置 */

async function patchSettings(patch: Partial<Settings>) {
  const previous = { ...settings.value };
  settings.value = { ...settings.value, ...patch };
  try {
    if ("alwaysOnTop" in patch) await bridge.setAlwaysOnTop(!!patch.alwaysOnTop);
    if ("launchAtStartup" in patch) {
      const applied = await bridge.setAutostart(!!patch.launchAtStartup);
      settings.value = { ...settings.value, launchAtStartup: applied };
    }
  } catch (err) {
    settings.value = previous;
    todos.say(String(err));
  }
}

async function applyHotkey(accelerator: string) {
  try {
    await bridge.setHotkey(accelerator);
    hotkeyError.value = null;
    settings.value = { ...settings.value, hotkey: accelerator };
  } catch (err) {
    hotkeyError.value = String(err);
  }
}

async function openDataDir() {
  try {
    await bridge.openDataDir();
  } catch (err) {
    todos.say(String(err));
  }
}

/* ------------------------------------------------------------ 键盘 */

function escape() {
  if (settingsOpen.value) {
    settingsOpen.value = false;
    return;
  }
  if (editingId.value) {
    cancelEdit();
    return;
  }
  if (selectedId.value) {
    focusCapture();
    return;
  }
  void hide();
}

function onKeydown(event: KeyboardEvent) {
  if (event.isComposing) return;

  const target = event.target as HTMLElement | null;
  const inField = !!target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA");
  const mod = event.ctrlKey || event.metaKey;

  if (mod && !event.altKey && event.key.toLowerCase() === "z") {
    event.preventDefault();
    todos.undo();
    return;
  }

  if (event.key === "Escape") {
    event.preventDefault();
    escape();
    return;
  }

  if (settingsOpen.value || editingId.value) return;

  if (event.altKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
    if (!selectedId.value) return;
    event.preventDefault();
    const id = selectedId.value;
    todos.move(id, event.key === "ArrowUp" ? -1 : 1);
    reveal(visibleTasks.value.findIndex((t) => t.id === id));
    return;
  }

  if ((event.key === "Delete" || event.key === "Backspace") && selectedId.value && !inField) {
    event.preventDefault();
    removeTask(selectedId.value);
    return;
  }

  if (!inField && selectedId.value) {
    if (event.key === " ") {
      event.preventDefault();
      todos.toggle(selectedId.value);
      return;
    }
    if (event.key === "e" || event.key === "E" || event.key === "Enter") {
      event.preventDefault();
      startEdit(selectedId.value);
      return;
    }
  }

  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    if (inField) return; // 召唤行自己会派发 ↑↓
    event.preventDefault();
    step(event.key === "ArrowDown" ? 1 : -1);
    return;
  }

  // 手不在输入框里的时候，直接敲字就开始记
  if (!inField && !mod && !event.altKey && event.key.length === 1 && event.key !== " ") {
    event.preventDefault();
    capture.value?.seed(event.key);
  }
}

/* ------------------------------------------------------------ 生命周期 */

function onWindowBlur() {
  void todos.flush();
}

function onVisibility() {
  if (document.visibilityState === "hidden") void todos.flush();
}

watch(() => settings.value.theme, applyTheme);
watch(visibleTasks, () => void nextTick(updateScroll));

// 选中的那一行可能从列表里消失（例如“已完成不留在此列表”时被划掉）：
// 那就把光标还给召唤行，否则底部图例在说一个看不见的选中项。
watch(visibleTasks, (items) => {
  if (selectedId.value && !items.some((t) => t.id === selectedId.value)) {
    focusCapture();
  }
});

onMounted(async () => {
  today.value = formatToday();
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("blur", onWindowBlur);
  document.addEventListener("visibilitychange", onVisibility);
  dark.addEventListener("change", applyTheme);

  stopShown = await bridge.onShown(onShown);
  stopNotice = await bridge.onNotice((text) => {
    hotkeyError.value = text;
    todos.say(text);
  });

  await todos.load();
  applyTheme();
  await nextTick();
  summon();
  updateScroll();
  capture.value?.focus();

  // 启动期间的提示（例如快捷键被别的程序占用）在这里补收一次；
  // 它会一直留在设置里，直到用户成功换绑。
  const pending = await bridge.takeNotice();
  if (pending) {
    hotkeyError.value = pending;
    todos.say(pending);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("blur", onWindowBlur);
  document.removeEventListener("visibilitychange", onVisibility);
  dark.removeEventListener("change", applyTheme);
  stopShown?.();
  stopNotice?.();
});
</script>

<template>
  <div ref="panel" class="panel">
    <AppHead
      :date="today"
      :open-count="openCount"
      :has-tasks="tasks.length > 0"
      @settings="settingsOpen = !settingsOpen"
    />

    <SettingsSheet
      v-if="settingsOpen"
      :settings="settings"
      :hotkey-error="hotkeyError"
      @patch="patchSettings"
      @hotkey="applyHotkey"
      @open-data-dir="openDataDir"
      @close="settingsOpen = false"
    />

    <template v-else>
      <div class="body">
        <CaptureRow ref="capture" @submit="addTask" @step="step" @escape="hide" />

        <div class="listwrap">
          <span class="listwrap__fade" :class="{ 'is-on': scrolled }" aria-hidden="true"></span>
          <span
            class="listwrap__fade listwrap__fade--bottom"
            :class="{ 'is-on': overflowing && !atBottom }"
            aria-hidden="true"
          ></span>
          <div ref="list" class="list scroll" @scroll="onScroll" @click.self="focusCapture">
            <TransitionGroup name="settle" tag="ul" class="items">
              <template v-for="row in displayRows" :key="row.kind === 'task' ? row.task.id : '__done'">
                <TaskRow
                  v-if="row.kind === 'task'"
                  :task="row.task"
                  :selected="row.task.id === selectedId"
                  :editing="row.task.id === editingId"
                  @select="select(row.task.id)"
                  @toggle="todos.toggle(row.task.id)"
                  @edit="startEdit(row.task.id)"
                  @commit="(title: string) => commitEdit(row.task.id, title)"
                  @cancel="cancelEdit"
                  @remove="removeTask(row.task.id)"
                />
                <li v-else class="group">
                  <span>已完成 {{ row.count }}</span>
                  <button type="button" class="group__clear" @click="todos.clearDone">清除</button>
                </li>
              </template>
            </TransitionGroup>

            <div v-if="ready && !visibleTasks.length" class="empty">
              <span class="empty__lead">清单是空的。</span>
              <span class="empty__note">在上面写下一件事，按回车记下。</span>
            </div>
          </div>
        </div>
      </div>

      <HintBar :message="message" :selected="!!selectedId" @undo="todos.undo" />
    </template>
  </div>
</template>

<style scoped>
.body {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
}

.listwrap {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
}

/* 内容被裁切的地方用一点渐变收口，提示还能滚 */
.listwrap__fade {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 16px;
  background: linear-gradient(to bottom, var(--bg), transparent);
  opacity: 0;
  transition: opacity 150ms linear;
  pointer-events: none;
  z-index: 2;
}

.listwrap__fade.is-on {
  opacity: 1;
}

.listwrap__fade--bottom {
  top: auto;
  bottom: 0;
  background: linear-gradient(to top, var(--bg), transparent);
}

.list {
  flex: 1 1 auto;
  min-height: 0;
  padding: 0 8px 12px;
}

.items {
  position: relative;
  margin: 0;
  padding: 0;
  list-style: none;
}

/* 「已完成」这一组的标签：结构信息，不是装饰 */
.group {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 10px 6px;
  font-size: var(--t-micro);
  font-weight: 500;
  color: var(--ink-3);
}

.group__clear {
  font-size: var(--t-micro);
  font-weight: 600;
  color: var(--accent);
}

.group__clear:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}

.empty {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  /* 与列表里的正文对齐：8px 列表内边距 + 8px 行内边距 + 26px 方框 + 10px 间距 = 52px */
  padding: 24px 12px 0 44px;
}

.empty__lead {
  font-size: var(--t-label);
  font-weight: 500;
  color: var(--ink-2);
}

.empty__note {
  font-size: var(--t-note);
  color: var(--ink-3);
}
</style>

<style>
/* 列表的动效：只在动作发生的地方发生
   —— 记下一件事（进入）、划掉一件事（沉底）、删掉一件事（离开） */
.settle-move {
  transition: transform 240ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.settle-enter-active {
  transition:
    opacity 160ms linear,
    transform 160ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.settle-enter-from {
  opacity: 0;
  transform: translateY(-6px);
}

.settle-leave-active {
  position: absolute;
  left: 0;
  right: 0;
  transition: opacity 130ms linear;
}

.settle-leave-to {
  opacity: 0;
}
</style>
