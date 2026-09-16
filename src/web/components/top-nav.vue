<script setup lang="ts">
import { t, type Locale } from '../i18n.ts';
import type { Theme } from '../preferences.ts';
import type { ConnectionStatus } from '../types.ts';
import { AppIcon } from './app-icon.vue';
import { Tooltip } from './ui/Tooltip.vue';
import {
  IconGlobe,
  IconMoon,
  IconPower,
  IconRefresh,
  IconSun,
} from './icons.vue';

type TopNavProps = {
  locale: Locale;
  theme: Theme;
  status: ConnectionStatus;
  activeCreator: string;
  onThemeToggle: () => void;
  onLocaleToggle: () => void;
  onReconnect: () => void;
  onDisconnect: () => void;
};

const props = defineProps<TopNavProps>();
</script>

<template>
  <header class="top-nav">
    <div class="brand-section">
      <Tooltip text="TikTok LIVE" position="bottom">
        <div class="brand-logo">
          <AppIcon :size="28" />
        </div>
      </Tooltip>
      <div class="brand-info">
        <h1>
          TikTok LIVE
          <span :class="['badge-live', props.status === 'connected' ? 'live' : props.status === 'connecting' || props.status === 'retrying' ? 'busy' : 'offline']">
            {{ props.status === 'connected' ? t(props.locale, 'live') : props.status === 'connecting' || props.status === 'retrying' ? t(props.locale, 'connecting') : t(props.locale, 'disconnected') }}
          </span>
        </h1>
      </div>
    </div>

    <div class="top-center">
      <Tooltip v-if="props.activeCreator" :text="`Status: ${props.status}`" position="bottom">
        <div class="active-creator-pill">
          <span :class="['status-dot', props.status === 'connected' ? 'online' : props.status === 'connecting' || props.status === 'retrying' ? 'busy' : 'offline']" />
          <span>@{{ props.activeCreator.replace(/^@/, '') }}</span>
        </div>
      </Tooltip>
    </div>

    <div class="top-actions">
      <template v-if="props.status === 'connected'">
        <Tooltip :text="t(props.locale, 'reconnect')" position="bottom">
          <button
            class="btn-icon"
            type="button"
            @click="props.onReconnect"
          >
            <IconRefresh />
          </button>
        </Tooltip>
        <Tooltip :text="t(props.locale, 'disconnect')" position="bottom">
          <button
            class="btn-icon btn-danger"
            type="button"
            @click="props.onDisconnect"
          >
            <IconPower />
          </button>
        </Tooltip>
      </template>

      <Tooltip :text="t(props.locale, 'switchTheme')" position="bottom">
        <button
          class="btn-icon"
          type="button"
          @click="props.onThemeToggle"
        >
          <IconSun v-if="props.theme === 'dark'" />
          <IconMoon v-else />
        </button>
      </Tooltip>

      <Tooltip :text="`${t(props.locale, 'switchLanguage')} (${props.locale.toUpperCase()})`" position="bottom">
        <button
          class="btn-icon"
          type="button"
          :aria-label="`${t(props.locale, 'switchLanguage')} (${props.locale.toUpperCase()})`"
          @click="props.onLocaleToggle"
        >
          <IconGlobe />
        </button>
      </Tooltip>
    </div>
  </header>
</template>
