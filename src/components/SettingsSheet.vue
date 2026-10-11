<script setup lang="ts">
/**
 * 设置。占满书写区的一页，顶部的仪器行留在外面，
 * 所以「设置」始终只是这页纸的背面，而不是一个浮在空中的对话框。
 */
import { computed, ref } from "vue";
import type { Settings, ThemeName } from "../types";
import ToggleSwitch from "./ToggleSwitch.vue";

const props = defineProps<{
  settings: Settings;
  hotkeyError: string | null;
}>();

const emit = defineEmits<{
  (e: "patch", patch: Partial<Settings>): void;
  (e: "hotkey", accelerator: string): void;
  (e: "openDataDir"): void;
  (e: "close"): void;
}>();

const capturing = ref(false);
const invalid = ref<string | null>(null);
const keycap = ref<HTMLButtonElement | null>(null);

const themes: { value: ThemeName; label: string }[] = [
  { value: "system", label: "跟随系统" },
  { value: "paper", label: "浅色" },
  { value: "night", label: "深色" },
];

const pretty = computed(() => props.settings.hotkey.replace(/\+/g, " + "));

const NAMED: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Escape: "Escape",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return code.slice(6);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  return NAMED[code] ?? null;
}

function startCapture() {
  invalid.value = null;
  capturing.value = true;
  keycap.value?.focus();
}

function onCaptureKeydown(event: KeyboardEvent) {
  if (!capturing.value) return;
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    capturing.value = false;
    invalid.value = null;
    return;
  }

  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Super");

  const key = keyName(event.code);
  if (!key) return; // 只按下了修饰键，继续等

  if (!modifiers.some((m) => m !== "Shift")) {
    invalid.value = "至少要配一个 Ctrl 或 Alt，否则会挡住正常的打字。";
    return;
  }

  invalid.value = null;
  capturing.value = false;
  emit("hotkey", [...modifiers, key].join("+"));
}
</script>

<template>
  <section class="sheet">
    <header class="sheet__head">
      <h2 class="sheet__title">设置</h2>
      <button type="button" class="sheet__done" @click="emit('close')">完成</button>
    </header>

    <div class="sheet__body scroll">
      <!-- 最要紧的一项，单独一行 -->
      <div class="field field--stack">
        <span class="field__label">召唤快捷键</span>
        <span class="field__note">在任何一个界面按下它，面板就出现在最前面；再按一次收起。</span>
        <button
          ref="keycap"
          type="button"
          class="keycap"
          :class="{ 'is-capturing': capturing }"
          @click="startCapture"
          @keydown="onCaptureKeydown"
          @blur="capturing = false"
        >
          {{ capturing ? "按下新的组合键…" : pretty }}
        </button>
        <span v-if="invalid || hotkeyError" class="field__error">{{ invalid ?? hotkeyError }}</span>
      </div>

      <div class="field">
        <span class="field__text">
          <span class="field__label">失去焦点时自动收起</span>
          <span class="field__note">点到别的窗口，它就自己让开。</span>
        </span>
        <ToggleSwitch
          :model-value="settings.hideOnBlur"
          label="失去焦点时自动收起"
          @update:model-value="emit('patch', { hideOnBlur: $event })"
        />
      </div>

      <div class="field">
        <span class="field__text">
          <span class="field__label">始终显示在最前面</span>
          <span class="field__note">让它浮在其他窗口上面。</span>
        </span>
        <ToggleSwitch
          :model-value="settings.alwaysOnTop"
          label="始终显示在最前面"
          @update:model-value="emit('patch', { alwaysOnTop: $event })"
        />
      </div>

      <div class="field">
        <span class="field__text">
          <span class="field__label">开机时自动启动</span>
          <span class="field__note">开机后自己起来，等你的快捷键。</span>
        </span>
        <ToggleSwitch
          :model-value="settings.launchAtStartup"
          label="开机时自动启动"
          @update:model-value="emit('patch', { launchAtStartup: $event })"
        />
      </div>

      <div class="field">
        <span class="field__text">
          <span class="field__label">已完成的事项留在列表里</span>
          <span class="field__note">关掉之后，划完就收走。</span>
        </span>
        <ToggleSwitch
          :model-value="settings.showCompleted"
          label="已完成的事项留在列表里"
          @update:model-value="emit('patch', { showCompleted: $event })"
        />
      </div>

      <div class="field">
        <span class="field__text">
          <span class="field__label">外观</span>
        </span>
        <span class="seg">
          <button
            v-for="theme in themes"
            :key="theme.value"
            type="button"
            class="seg__item"
            :class="{ 'is-on': settings.theme === theme.value }"
            :aria-pressed="settings.theme === theme.value"
            @click="emit('patch', { theme: theme.value })"
          >
            {{ theme.label }}
          </button>
        </span>
      </div>

      <div class="field field--last">
        <span class="field__text">
          <span class="field__label">待办存在哪里</span>
          <span class="field__note">一个 JSON 文件，随时可以自己备份。</span>
        </span>
        <button type="button" class="field__btn" @click="emit('openDataDir')">打开文件夹</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sheet {
  display: flex;
  flex-direction: column;
  flex: 1 1 auto;
  min-height: 0;
}

.sheet__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex: none;
  height: 46px;
  padding: 0 16px;
}

.sheet__title {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.005em;
  color: var(--ink);
}

.sheet__done {
  font-size: var(--t-note);
  font-weight: 600;
  color: var(--accent);
}

.sheet__done:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}

.sheet__body {
  flex: 1 1 auto;
  min-height: 0;
  padding: 0 16px 20px;
}

.field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 13px 0;
  border-bottom: 1px solid var(--line);
}

.field--last {
  border-bottom: 0;
}

.field--stack {
  display: flex;
  align-items: flex-start;
  flex-direction: column;
  gap: 5px;
  padding-top: 6px;
}

.field__label {
  display: block;
  font-size: var(--t-label);
  font-weight: 500;
  letter-spacing: -0.003em;
  color: var(--ink);
}

.field__note {
  display: block;
  margin-top: 3px;
  font-size: var(--t-note);
  line-height: 1.5;
  color: var(--ink-2);
}

.field--stack .field__note {
  margin-top: 0;
}

.field__error {
  display: block;
  margin-top: 2px;
  font-size: var(--t-note);
  line-height: 1.5;
  color: var(--seal);
}

.keycap {
  margin-top: 7px;
  padding: 7px 12px;
  border: 1px solid var(--line);
  border-bottom-width: 2px;
  border-radius: var(--r-btn);
  background: var(--raise);
  font-family: var(--font-display);
  font-size: 13px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.01em;
  color: var(--ink);
  transition:
    border-color 120ms linear,
    color 120ms linear;
}

.keycap:hover {
  border-color: var(--line-strong);
}

.keycap.is-capturing {
  border-color: var(--accent);
  color: var(--accent);
}

.seg {
  display: flex;
  flex: none;
  gap: 2px;
  padding: 2px;
  border-radius: 10px;
  background: var(--hover);
}

.seg__item {
  padding: 5px 10px;
  border-radius: 8px;
  font-size: var(--t-note);
  font-weight: 500;
  color: var(--ink-3);
  transition:
    background-color 120ms linear,
    color 120ms linear;
}

.seg__item:hover {
  color: var(--ink-2);
}

.seg__item.is-on {
  background: var(--raise);
  box-shadow: var(--raise-shadow);
  color: var(--ink);
  font-weight: 600;
}

.field__btn {
  flex: none;
  font-size: var(--t-note);
  font-weight: 600;
  color: var(--accent);
}

.field__btn:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
