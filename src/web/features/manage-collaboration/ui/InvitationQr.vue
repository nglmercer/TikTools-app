<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
const props = defineProps<{ value: string }>();
const canvas = ref<HTMLCanvasElement>();
const failed = ref(false);
const { t } = useI18n();
let generation = 0;
watch([() => props.value, canvas], async ([value, target]) => {
  const epoch = ++generation; failed.value = false;
  if (!target || !value) return;
  try {
    const QRCode = await import('qrcode');
    if (epoch !== generation) return;
    await QRCode.toCanvas(target, value, { errorCorrectionLevel: 'M', width: 260, margin: 4 });
  } catch { if (epoch === generation) failed.value = true; }
}, { immediate: true });
onBeforeUnmount(() => { generation += 1; if (canvas.value) { canvas.value.width = 0; canvas.value.height = 0; } });
</script>
<template>
  <p
    v-if="failed"
    role="alert"
  >
    {{ t('collaboration.qrFailed') }}
  </p>
  <canvas
    v-else
    ref="canvas"
    :aria-label="t('collaboration.privateLink')"
    role="img"
  />
</template>
