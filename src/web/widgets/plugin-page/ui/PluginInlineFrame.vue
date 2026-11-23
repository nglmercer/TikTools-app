<script setup lang="ts">
import { onBeforeUnmount, onMounted, useTemplateRef, watch } from 'vue';
import {
  PluginWebviewHost,
  type BrokerBackend,
  type BrokerEvent,
  type BrokerResponse,
} from '@/entities/plugin/api/plugin-webview-host.ts';

const props = defineProps<{
  pluginId: string;
  src: string;
  backend: BrokerBackend;
  subscribeTopic?: (topic: string, listener: (data: unknown) => void) => () => void;
}>();

const frame = useTemplateRef('frame');
let host: PluginWebviewHost | null = null;

function disposeHost(): void {
  host?.dispose();
  host = null;
}

function mountHost(): void {
  disposeHost();
  const element = frame.value;
  if (!element) return;
  host = new PluginWebviewHost(element.contentWindow, {
    pluginId: props.pluginId,
    backend: props.backend,
    postToFrame: (envelope: BrokerResponse | BrokerEvent) => element.contentWindow?.postMessage(envelope, '*'),
    subscribeTopic: props.subscribeTopic,
  });
}

function onMessage(event: MessageEvent): void {
  if (!host || event.source !== (frame.value?.contentWindow ?? null)) return;
  void host.handleFrameMessage(event.data, event.source);
}

watch([() => props.src, () => props.pluginId, () => props.backend], mountHost);

onMounted(() => {
  window.addEventListener('message', onMessage);
  mountHost();
});

onBeforeUnmount(() => {
  window.removeEventListener('message', onMessage);
  disposeHost();
});
</script>

<template>
  <iframe
    ref="frame"
    class="plugin-frame"
    :title="pluginId"
    :src="src"
    sandbox="allow-scripts allow-popups allow-top-navigation-to-custom-protocols"
    referrerpolicy="no-referrer"
  />
</template>

<style scoped>
.plugin-frame {
  display: block;
  width: 100%;
  height: 100%;
  border: 0;
  background: var(--color-bg-app);
}
</style>
