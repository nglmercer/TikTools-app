<script setup lang="ts">
import { computed, type Component } from 'vue';
import { t, type Locale } from '../i18n.ts';
import type { AppTab } from '../types.ts';
import { Tooltip } from './ui/Tooltip.vue';
import {
  IconBarChart,
  IconChat,
  IconCoins,
  IconRadio,
  IconSettings,
  IconSparkles,
  IconPlugins,
} from './icons.vue';

type NavigationRailProps = {
  locale: Locale;
  activeTab: AppTab;
  onTabChange: (tab: AppTab) => void;
};

const props = defineProps<NavigationRailProps>();

type NavigationTab = {
  id: AppTab;
  tooltip: string;
  icon: Component;
};

const navTabs = computed<NavigationTab[]>(() => [
  { id: 'feed', tooltip: t(props.locale, 'tabFeed'), icon: IconChat },
  { id: 'points', tooltip: t(props.locale, 'tabPoints'), icon: IconCoins },
  { id: 'analytics', tooltip: t(props.locale, 'tabAnalytics'), icon: IconBarChart },
  { id: 'connect', tooltip: t(props.locale, 'tabConnect'), icon: IconRadio },
  { id: 'behavior', tooltip: t(props.locale, 'tabBehavior'), icon: IconSparkles },
  { id: 'plugins', tooltip: t(props.locale, 'tabPlugins'), icon: IconPlugins },
  { id: 'settings', tooltip: t(props.locale, 'tabSettings'), icon: IconSettings },
]);
</script>

<template>
  <nav class="nav-rail" aria-label="Main Navigation">
    <Tooltip
      v-for="tab in navTabs"
      :key="tab.id"
      :text="tab.tooltip"
      position="right"
    >
      <button
        type="button"
        :class="['nav-tab-btn', { active: props.activeTab === tab.id }]"
        :aria-label="tab.tooltip"
        :aria-current="props.activeTab === tab.id ? 'page' : undefined"
        @click="props.onTabChange(tab.id)"
      >
        <component :is="tab.icon" />
      </button>
    </Tooltip>
  </nav>
</template>
