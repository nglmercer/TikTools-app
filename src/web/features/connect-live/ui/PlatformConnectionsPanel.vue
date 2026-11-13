<script setup lang="ts">
import { onMounted, onUnmounted, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { createPlatformConnections } from '@/entities/connection/model/platform-connections.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const { t } = useI18n();
const state = reactive(createPlatformConnections(useControlClient()));
const fields = reactive<Record<string, { target: string; token: string }>>({});
function form(platform: string) { return fields[platform] ??= { target: '', token: '' }; }
async function connect(platform: typeof state.platforms[number]['platform']) {
  const entry = form(platform);
  const token = entry.token;
  entry.token = '';
  await state.connect(platform, entry.target, token);
}
onMounted(() => { void state.refresh(); });
onUnmounted(state.dispose);
</script>

<template>
  <BasePanel
    v-if="state.additionalPlatforms.length || state.error"
    class="platform-connections"
    :title="t('livePlatforms.title')"
  >
    <p
      v-if="state.error"
      role="alert"
    >
      {{ state.error }}
    </p>
    <p v-if="!state.additionalPlatforms.length">
      {{ t('livePlatforms.unavailable') }}
    </p>
    <div
      v-for="provider in state.additionalPlatforms"
      :key="provider.platform"
      class="platform-provider"
    >
      <h3>{{ provider.name }}</h3>
      <p>{{ t('livePlatforms.tokenHint') }}</p>
      <div class="platform-capabilities">
        <span v-if="provider.capabilities.chat">{{ t('livePlatforms.chat') }}</span>
        <span v-if="provider.capabilities.subscriptions">{{ t('livePlatforms.subscriptions') }}</span>
        <span v-if="provider.capabilities.gifts">{{ t('livePlatforms.gifts') }}</span>
      </div>
      <form
        class="platform-form"
        @submit.prevent="connect(provider.platform)"
      >
        <BaseTextField
          :id="`platform-target-${provider.platform}`"
          v-model="form(provider.platform).target"
          :aria-label="t('livePlatforms.channel')"
          :placeholder="t('livePlatforms.channel')"
          :disabled="state.busy"
          autocomplete="off"
        />
        <BaseTextField
          :id="`platform-token-${provider.platform}`"
          v-model="form(provider.platform).token"
          type="password"
          :aria-label="t('livePlatforms.token')"
          :placeholder="t('livePlatforms.token')"
          :disabled="state.busy"
          autocomplete="off"
        />
        <BaseButton
          type="submit"
          variant="primary"
          :disabled="state.busy || !form(provider.platform).target.trim() || !form(provider.platform).token.trim()"
        >
          {{ t('connection.connect') }}
        </BaseButton>
      </form>
      <ul class="platform-sessions">
        <li
          v-for="connection in state.connections.filter((item) => item.platform === provider.platform)"
          :key="connection.connectionId"
        >
          <span>{{ connection.channel?.handle || connection.target }} — {{ t(`livePlatforms.states.${connection.state}`) }}</span>
          <span
            v-if="connection.error"
            role="alert"
          >{{ connection.error.message }}</span>
          <BaseButton
            variant="danger"
            @click="state.disconnect(connection.connectionId)"
          >
            {{ t('connection.disconnect') }}
          </BaseButton>
        </li>
      </ul>
    </div>
  </BasePanel>
</template>

<style scoped>
.platform-connections { flex: none; }
.platform-connections :deep(.base-panel__body) { flex: none; overflow: visible; }
.platform-provider h3, .platform-provider p { margin: 0; }
.platform-provider, .platform-sessions, .platform-sessions li { display: flex; flex-direction: column; gap: var(--space-3); }
.platform-form { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.platform-form > :not(button) { flex: 1; min-width: 180px; }
.platform-capabilities { display: flex; gap: var(--space-3); color: var(--color-text-muted); }
.platform-sessions { list-style: none; padding: 0; }
.platform-sessions li { flex-direction: row; flex-wrap: wrap; align-items: center; }
</style>
