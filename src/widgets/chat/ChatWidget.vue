<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import WidgetLayerStack from '../sdk/WidgetLayerStack.vue';
import { useWidgetText, useWidgetTextOrder, widgetDesignKey } from '../sdk/text.ts';
const text = useWidgetText('chat');
const order = useWidgetTextOrder('chat');
import { computed, inject, ref } from 'vue';
import type { ChatWidgetSettings } from '../shared/config.ts';
import type { ChatMessageView } from '../shared/chat-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type ChatWidgetProps = {
  messages: ChatMessageView[];
  status: GatewayStatus;
  settings: ChatWidgetSettings;
  debug: boolean;
};

const props = defineProps<ChatWidgetProps>();
const design = inject(widgetDesignKey, undefined);
const layered = computed(() => design?.value.layers !== undefined);

const failedAvatars = ref<Record<string, boolean>>({});

function showAvatar(message: ChatMessageView): boolean {
  return props.settings.showAvatars && !!message.avatarUrl && !failedAvatars.value[message.id];
}

function initialsFor(message: ChatMessageView): string {
  const clean = message.uniqueId.replace(/^@+/, '');
  return clean.slice(0, 2).toUpperCase() || '•';
}
</script>

<template>
  <WidgetStage :debug="props.debug" :status="props.status" align="bottom" class="chat-stage">
    <TransitionGroup name="chat" tag="div" class="chat-list" aria-live="polite">
      <div v-for="message in props.messages" :key="message.id" class="chat-message">
        <WidgetLayerStack v-if="layered" :layers="design?.layers ?? []" :event="message"
          :avatar-url="props.settings.showAvatars ? message.avatarUrl ?? undefined : undefined" />
        <template v-else>
        <div class="chat-avatar" aria-hidden="true">
          <img
            v-if="showAvatar(message)"
            :src="message.avatarUrl as string"
            :alt="message.displayName"
            referrerpolicy="no-referrer"
            @error="failedAvatars[message.id] = true"
          />
          <span v-else>{{ initialsFor(message) }}</span>
        </div>
        <div class="chat-content">
          <span v-if="text('name', message)" class="chat-name" :style="{ order: order('name') }">{{ text('name', message) }}</span>
          <span v-if="text('message', message)" class="chat-text" :style="{ order: order('message') }">{{ text('message', message) }}</span>
        </div>
        </template>
      </div>
    </TransitionGroup>
  </WidgetStage>
</template>

<style scoped src="./chat.css"></style>
