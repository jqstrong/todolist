<script setup lang="ts">
/**
 * 底部图例行 / 状态行。
 * 平时它教你怎么用键盘；刚做完一件事时，它变成那件事的收据（可撤销）。
 */
import type { StatusMessage } from "../composables/useTodos";

defineProps<{
  message: StatusMessage | null;
  selected: boolean;
}>();

const emit = defineEmits<{
  (e: "undo"): void;
}>();
</script>

<template>
  <footer class="hint">
    <span class="hint__left">
      <template v-if="message">
        <span class="hint__msg">{{ message.text }}</span>
        <button v-if="message.undo" type="button" class="hint__act" @click="emit('undo')">撤销</button>
      </template>
    </span>

    <span class="hint__keys">
      <template v-if="selected">
        <b>空格</b> 完成 <b>E</b> 编辑 <b>DEL</b> 删除
      </template>
      <template v-else>
        <b>回车</b> 记下 <b>↑↓</b> 选择 <b>ESC</b> 收起
      </template>
    </span>
  </footer>
</template>

<style scoped>
.hint {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  flex: none;
  height: 36px;
  padding: 0 16px;
  border-top: 1px solid var(--line);
  font-size: var(--t-micro);
  font-weight: 450;
  overflow: hidden;
  white-space: nowrap;
  color: var(--ink-3);
}

.hint__left,
.hint__keys {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  overflow: hidden;
}

.hint__keys {
  flex: none;
}

.hint__keys b {
  font-weight: 600;
  color: var(--ink-2);
}

.hint__msg {
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--ink-2);
}

.hint__act {
  flex: none;
  font-size: var(--t-micro);
  font-weight: 600;
  color: var(--accent);
}

.hint__act:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
