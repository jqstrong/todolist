<script setup lang="ts">
/**
 * 设置里的开关。现代控件语言：轨道 + 圆钮。
 * （列表里那个方框加勾，是「这件事完成了」；这里是「这个选项开着」，
 *   两件事语义不同，用两套各自的通用控件更不容易误读。）
 */
defineProps<{ modelValue: boolean; label: string }>();

const emit = defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <button
    type="button"
    class="switch"
    :class="{ 'is-on': modelValue }"
    role="switch"
    :aria-checked="modelValue"
    @click="emit('update:modelValue', !modelValue)"
  >
    <span class="sr-only">{{ label }}</span>
    <span class="switch__track" aria-hidden="true"><span class="switch__knob"></span></span>
  </button>
</template>

<style scoped>
.switch {
  display: grid;
  place-items: center;
  flex: none;
  width: 40px;
  height: 32px;
  border-radius: var(--r-btn);
}

.switch__track {
  position: relative;
  display: block;
  width: 38px;
  height: 22px;
  border-radius: 11px;
  background: var(--line-strong);
  transition: background-color 160ms var(--ease);
}

.switch__knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fff;
  box-shadow: var(--thumb-shadow);
  transition: transform 160ms var(--ease);
}

.switch.is-on .switch__track {
  background: var(--accent);
}

.switch.is-on .switch__knob {
  transform: translateX(16px);
}

.switch:hover .switch__track {
  filter: brightness(0.97);
}
</style>
