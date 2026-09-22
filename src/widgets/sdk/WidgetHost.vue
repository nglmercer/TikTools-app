<script setup lang="ts">
import { computed, onMounted, onUnmounted, provide, shallowRef, watch } from 'vue';
import { widgetDesignKey } from './text.ts';
import { removeDefaultHeadingLayers } from './layers.ts';
import { createWidgetRuntime, type WidgetRuntime } from '../shared/widget-runtime.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import { resolveWidgetDesign, styleVariables, type WidgetStyle, type WidgetTemplate } from './template.ts';
import type { WidgetClock } from '../shared/clock.ts';
import type { DomainEventEnvelope } from '../shared/event-types.ts';
import { makeTestFollowEnvelope, makeTestGiftEnvelope, makeTestGiftCombo, makeTestChatEnvelope, makeTestShareEnvelope, makeTestSubscribeEnvelope, mountWidgetTestHook, WIDGET_TEST_HOOK } from '../shared/test-events.ts';

const props = withDefaults(defineProps<{
  template: WidgetTemplate;
  mode?: 'live' | 'preview';
  search?: string;
  design?: WidgetStyle;
  replayKey?: number;
  debug?: boolean;
  testHook?: boolean;
}>(), { mode: 'live', search: '', replayKey: 0, debug: false, testHook: false });

// Keep the final sample visible for editing, without changing live timing.
const previewClock: WidgetClock = { now: () => Date.now(), setTimeout: () => null, clearTimeout: () => {} };
const instance = shallowRef(props.template.create(props.search, props.mode === 'preview' ? previewClock : undefined));
const status = shallowRef<GatewayStatus>('idle');
const generation = shallowRef(0);
// Every consumer renders the same effective design. The editor and the
// dashboard may pass partial drafts, while OBS may pass a design from the URL;
// defaults always come from the template schema first.
const design = computed(() => removeDefaultHeadingLayers(props.template.id, resolveWidgetDesign(props.template, props.design)));
const variables = computed(() => styleVariables(design.value));
provide(widgetDesignKey, design);
let runtime: WidgetRuntime | undefined;
let timers: ReturnType<typeof setTimeout>[] = [];
let mounted = false;
let removeHook: (() => void) | undefined;

function stop() {
  timers.forEach(clearTimeout);
  timers = [];
  runtime?.stop();
  runtime = undefined;
  instance.value.controller.dispose();
  removeHook?.();
  removeHook = undefined;
}

function start() {
  stop();
  status.value = 'idle';
  instance.value = props.template.create(props.search, props.mode === 'preview' ? previewClock : undefined);
  // A new controller needs a fresh transition tree. Reusing it can leave the
  // previous card in the flex stage while the next sample enters.
  generation.value += 1;
  const controller = instance.value.controller;
  const inject = (envelope: DomainEventEnvelope) => controller.handleEnvelope(envelope);
  if (props.mode === 'preview') {
    props.template.samples().forEach((sample, index) => {
      timers.push(setTimeout(() => inject(sample), 100 + index * 150));
    });
  } else {
    runtime = createWidgetRuntime({ onEnvelope: inject, onStatus: (next) => { status.value = next; } });
    runtime.start();
  }
  if (props.testHook) {
    const api = {
      emitEnvelope: inject,
      emitTestFollow: (overrides?: Parameters<typeof makeTestFollowEnvelope>[0]) => inject(makeTestFollowEnvelope(overrides)),
      emitTestGift: (overrides?: Parameters<typeof makeTestGiftEnvelope>[0]) => inject(makeTestGiftEnvelope(overrides)),
      emitTestGiftCombo: (count: number, overrides?: Parameters<typeof makeTestGiftCombo>[1]) => { makeTestGiftCombo(count, overrides).forEach(inject); },
      emitTestChat: (overrides?: Parameters<typeof makeTestChatEnvelope>[0]) => inject(makeTestChatEnvelope(overrides)),
      emitTestShare: (overrides?: Parameters<typeof makeTestShareEnvelope>[0]) => inject(makeTestShareEnvelope(overrides)),
      emitTestSubscribe: (overrides?: Parameters<typeof makeTestSubscribeEnvelope>[0]) => inject(makeTestSubscribeEnvelope(overrides)),
      connectionStatus: () => status.value,
    };
    mountWidgetTestHook(api);
    removeHook = () => {
      const target = window as unknown as Record<string, unknown>;
      if (target[WIDGET_TEST_HOOK] === api) delete target[WIDGET_TEST_HOOK];
    };
  }
}

// A functional render bridge keeps template-specific state inside the SDK.
const Content = () => instance.value.render(status.value, props.debug);
onMounted(() => { mounted = true; start(); });
watch(() => [props.template, props.search, props.mode, props.replayKey], () => { if (mounted) start(); });
onUnmounted(stop);
</script>

<template>
  <div class="widget-host" :style="variables" :data-widget="props.template.id">
    <Content :key="generation" />
  </div>
</template>

<style scoped>
.widget-host { width: 100%; height: 100%; position: relative; overflow: hidden; isolation: isolate; color-scheme: dark; }
</style>
