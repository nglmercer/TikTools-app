<script setup lang="ts">
import { ref } from 'vue';
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
  <div class="chat-stage">
    <TransitionGroup name="chat" tag="div" class="chat-list" aria-live="polite">
      <div v-for="message in props.messages" :key="message.id" class="chat-message">
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
          <span class="chat-name">{{ message.displayName }}</span>
          <span class="chat-text">{{ message.text }}</span>
        </div>
      </div>
    </TransitionGroup>
    <div v-if="props.debug" class="widget-debug-status">gateway: {{ props.status }}</div>
  </div>
</template>
