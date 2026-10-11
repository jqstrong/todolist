<script setup lang="ts">
/**
 * 召唤行 —— 界面的主角。
 * 面板一出现，光标就在这里：想起一件事，写下来，回车，结束。
 */
import { ref } from "vue";

const emit = defineEmits<{
  (e: "submit", title: string): void;
  (e: "step", delta: -1 | 1): void;
  (e: "escape"): void;
}>();

const draft = ref("");
const field = ref<HTMLInputElement | null>(null);
const live = ref(false);

function submit() {
  const title = draft.value;
  if (!title.trim()) return;
  draft.value = "";
  emit("submit", title);
}

function onKeydown(event: KeyboardEvent) {
  if (event.isComposing) return;
  if (event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    submit();
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    event.stopPropagation();
    emit("step", 1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    event.stopPropagation();
    emit("step", -1);
  } else if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    // 先擦掉没写完的那半句，再收起面板
    if (draft.value) draft.value = "";
    else emit("escape");
  }
}

defineExpose({
  focus: () => field.value?.focus(),
  blur: () => {
    if (document.activeElement === field.value) field.value?.blur();
  },
  seed: (text: string) => {
    field.value?.focus();
    draft.value = text;
  },
});
</script>

<template>
  <div class="capture" :class="{ 'is-live': live }">
    <span class="capture__mark" aria-hidden="true">
      <svg viewBox="0 0 18 18" focusable="false">
        <path d="M9 3.8V14.2M3.8 9H14.2" />
      </svg>
    </span>

    <input
      ref="field"
      v-model="draft"
      class="capture__input"
      type="text"
      autocomplete="off"
      spellcheck="false"
      placeholder="记一件要做的事…"
      aria-label="新的待办"
      @keydown="onKeydown"
      @focus="live = true"
      @blur="live = false"
    />
  </div>
</template>

<style scoped>
.capture {
  --mark-ink: var(--ink-3);
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  flex: none;
  margin: 0 12px 8px;
  padding: 0 14px 0 8px;
  min-height: 46px;
  border: 1px solid var(--line);
  border-radius: var(--r-field);
  background: var(--field);
  box-shadow: 0 1px 2px rgba(16, 20, 28, 0.04);
  transition:
    border-color 130ms linear,
    box-shadow 130ms linear;
}

/* 光标在这里的时候，框子稍微“醒”一点——不用彩色描边，主角不该变成一只带边框的输入框 */
.capture.is-live {
  --mark-ink: var(--accent);
  border-color: var(--line-strong);
  box-shadow: 0 1px 3px rgba(16, 20, 28, 0.07);
}

.capture__mark {
  display: grid;
  place-items: center;
  transition: color 130ms linear;
  color: var(--mark-ink);
}

.capture__mark svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.6;
  stroke-linecap: round;
}

.capture__input {
  width: 100%;
  padding: 12px 0;
  font-size: var(--t-capture);
  font-weight: 400;
  line-height: 1.4;
  letter-spacing: -0.01em;
  color: var(--ink);
  caret-color: var(--accent);
  user-select: text;
}

.capture__input::placeholder {
  color: var(--ink-3);
}

.capture__input:focus-visible {
  outline: none;
}
</style>
