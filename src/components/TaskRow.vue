<script setup lang="ts">
/**
 * 一条待办。
 *
 * 完成时的动作仍然是这里最重要的动效：一道笔迹从左往右划过文字，方框同时被填满、
 * 勾被描出来。划线的做法是在正文之上叠一层同样的文字（透明字形 + 保留 text-decoration），
 * 用 clip-path 从右往左裁掉 / 放出——线是画在字之上的，折行也仍然对得齐。
 */
import { nextTick, ref, watch } from "vue";
import type { Task } from "../types";

const props = defineProps<{
  task: Task;
  selected: boolean;
  editing: boolean;
}>();

const emit = defineEmits<{
  (e: "select"): void;
  (e: "toggle"): void;
  (e: "edit"): void;
  (e: "commit", title: string): void;
  (e: "cancel"): void;
  (e: "remove"): void;
}>();

const draft = ref("");
const area = ref<HTMLTextAreaElement | null>(null);

watch(
  () => props.editing,
  async (on) => {
    if (!on) return;
    draft.value = props.task.title;
    await nextTick();
    const el = area.value;
    if (!el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    grow();
  },
);

/** 编辑框跟着内容长高，长句子也不会被截断 */
function grow() {
  const el = area.value;
  if (!el) return;
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
}

function onEditKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    emit("commit", draft.value);
  } else if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
  } else if (event.key === "Tab") {
    event.preventDefault();
    emit("commit", draft.value);
  }
}
</script>

<template>
  <li
    class="row"
    :class="{ 'is-done': task.done, 'is-selected': selected, 'is-editing': editing }"
    @click="emit('select')"
    @dblclick="emit('edit')"
  >
    <span class="row__bar" aria-hidden="true"></span>

    <span class="row__mark">
      <button
        type="button"
        class="mark"
        :aria-label="task.done ? `标记「${task.title}」为未完成` : `标记「${task.title}」为已完成`"
        :aria-pressed="task.done"
        @click.stop="emit('toggle')"
      >
        <svg viewBox="0 0 18 18" aria-hidden="true" focusable="false">
          <rect class="mark__box" x="1.5" y="1.5" width="15" height="15" rx="4.5" />
          <path class="mark__tick" pathLength="1" d="M5 9.4 7.6 12 13.1 6" />
        </svg>
      </button>
    </span>

    <span class="row__body">
      <textarea
        v-if="editing"
        ref="area"
        v-model="draft"
        class="row__input"
        rows="1"
        spellcheck="false"
        @input="grow"
        @keydown="onEditKeydown"
        @blur="emit('commit', draft)"
      ></textarea>
      <template v-else>
        <span class="row__title">{{ task.title }}</span>
        <span class="row__strike" aria-hidden="true">{{ task.title }}</span>
      </template>
    </span>

    <span class="row__acts">
      <button type="button" class="iconbtn row__del" :aria-label="`删除「${task.title}」`" @click.stop="emit('remove')">
        <svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true" focusable="false">
          <path d="M2.4 2.4 9.6 9.6M9.6 2.4 2.4 9.6" />
        </svg>
      </button>
    </span>
  </li>
</template>

<style scoped>
.row {
  /* 颜色靠自定义属性逐级下沉，避免后代选择器互相打擂台 */
  --mark-border: var(--mark-line);
  --mark-fill: transparent;
  --mark-tick: var(--raise);
  --bar-alpha: 0;
  --row-bg: transparent;
  --row-shadow: none;
  --title-ink: var(--ink);

  position: relative;
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr) auto;
  align-items: start;
  gap: 10px;
  min-height: 42px;
  padding: 9px 10px 9px 8px;
  border-radius: var(--r-row);
  background: var(--row-bg);
  box-shadow: var(--row-shadow);
  transition:
    background-color 110ms linear,
    box-shadow 110ms linear;
}

.row:hover {
  --row-bg: var(--hover);
}

.row.is-selected {
  --row-bg: var(--raise);
  --row-shadow: var(--raise-shadow);
  --mark-border: var(--ink-2);
  --bar-alpha: 1;
}

.row.is-editing {
  --row-bg: var(--raise);
  --row-shadow: var(--raise-shadow);
  --bar-alpha: 1;
}

/* 已完成的排在最前面：划掉之后它就该安静下来。放在最后，压过 hover / selected */
.row.is-done {
  --mark-border: transparent;
  --mark-fill: var(--accent);
  --mark-tick: var(--accent-ink);
  --title-ink: var(--ink-3);
}

/* 当前这一行左边的小竖条——稿纸那道页边线，缩到只剩「你在这里」 */
.row__bar {
  position: absolute;
  top: 9px;
  bottom: 9px;
  left: 0;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
  opacity: var(--bar-alpha);
  transition: opacity 120ms linear;
  pointer-events: none;
}

.row__mark {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 22.5px;
}

.mark {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: var(--r-mark);
}

.mark svg {
  width: 18px;
  height: 18px;
  overflow: visible;
}

.mark__box {
  fill: var(--mark-fill);
  stroke: var(--mark-border);
  stroke-width: 1.5;
  transition:
    fill 130ms linear,
    stroke 130ms linear;
}

.mark__tick {
  fill: none;
  stroke: var(--mark-tick);
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-dasharray: 1;
  stroke-dashoffset: 1;
  /* 方框先填满，勾随后描出来 */
  transition: stroke-dashoffset 180ms var(--ease) 90ms;
}

.row.is-done .mark__tick {
  stroke-dashoffset: 0;
}

.mark:hover .mark__box {
  stroke: var(--ink-2);
}

.row.is-done .mark:hover .mark__box {
  stroke: transparent;
}

.row__body {
  position: relative;
  min-width: 0;
}

.row__title,
.row__strike {
  display: block;
  font-size: var(--t-task);
  font-weight: 400;
  line-height: 1.5;
  letter-spacing: -0.003em;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
  user-select: text;
}

.row__title {
  color: var(--title-ink);
  transition: color 180ms linear;
}

/* 叠在正文之上的那道笔迹：字形透明，只留下划线。
   inset:0 铺满 .row__body，于是这一层与正文的内容框严丝合缝，折行位置一致。 */
.row__strike {
  position: absolute;
  inset: 0;
  color: transparent;
  text-decoration: line-through;
  text-decoration-color: var(--accent);
  text-decoration-thickness: 1.5px;
  clip-path: inset(0 100% 0 0);
  transition: clip-path 240ms var(--ease);
  pointer-events: none;
}

.row.is-done .row__strike {
  clip-path: inset(0 0 0 0);
}

.row.is-editing .row__strike {
  display: none;
}

/* 正在改的那一行：不再套输入框的焦点框，底色和左侧竖条已经说明了一切 */
.row__input {
  display: block;
  width: 100%;
  resize: none;
  overflow: hidden;
  font-size: var(--t-task);
  line-height: 1.5;
  letter-spacing: -0.003em;
  color: var(--ink);
  caret-color: var(--accent);
  user-select: text;
}

.row__input:focus-visible {
  outline: none;
}

.row__acts {
  display: flex;
  align-items: center;
  height: 22.5px;
  opacity: 0;
  transition: opacity 110ms linear;
}

.row:hover .row__acts,
.row.is-selected .row__acts,
.row:focus-within .row__acts {
  opacity: 1;
}

.row__del {
  width: 22px;
  height: 22px;
  border-radius: 6px;
}

.row__del svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
}

.row__del:hover {
  color: var(--seal);
  background: var(--hover);
}
</style>
