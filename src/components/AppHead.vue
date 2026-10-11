<script setup lang="ts">
/**
 * 顶部的仪器行：左边今天的日子，右边还有多少件，最右边是设置。
 * 这一行也是拖动窗口的把手。
 */
import { computed } from "vue";

const props = defineProps<{
  date: string;
  openCount: number;
  hasTasks: boolean;
}>();

const emit = defineEmits<{
  (e: "settings"): void;
}>();

const status = computed(() => {
  if (!props.hasTasks) return "";
  return props.openCount > 0 ? `还有 ${props.openCount} 件` : "全部完成";
});
</script>

<template>
  <header class="head" data-tauri-drag-region>
    <span class="head__date">{{ date }}</span>
    <span class="head__right">
      <span v-if="status" class="head__status chrome">{{ status }}</span>
      <button type="button" class="iconbtn" aria-label="设置" @click="emit('settings')">
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true" focusable="false">
          <path class="tune__line" d="M1.9 5.3h12.2M1.9 10.7h12.2" />
          <circle class="tune__knob" cx="6.2" cy="5.3" r="1.9" />
          <circle class="tune__knob" cx="10.3" cy="10.7" r="1.9" />
        </svg>
      </button>
    </span>
  </header>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex: none;
  height: 46px;
  padding: 0 10px 0 16px;
}

/* 把手上的字不该抢走拖动的点击 */
.head__date,
.head__status {
  pointer-events: none;
}

.head__date {
  font-family: var(--font-display);
  font-size: 13px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.005em;
  color: var(--ink);
}

.head__right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tune__line {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
}

.tune__knob {
  fill: var(--bg);
  stroke: currentColor;
  stroke-width: 1.5;
}
</style>
