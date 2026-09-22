<script setup lang="ts">
import { computed, type Component } from 'vue';
import { i18nText, t, type Locale } from '../i18n.ts';
import type { AppTab } from '../types.ts';
import type { PluginPageDescriptor } from '../../automation/behavior/types.ts';
import widgetsOverlayGraphic from '../assets/widgets-overlay.svg';
import { isConnectionOnlyPage, pluginNavId } from '../../automation/plugins/declarative.ts';
import { Tooltip } from './ui/Tooltip.vue';
import { Icon, readIconName } from './icons/index.ts';
import {
  IconBarChart,
  IconChat,
  IconCoins,
  IconRadio,
  IconSettings,
  IconEdgeNet,
  IconPlugins,
} from './icons.vue';

type NavigationRailProps = {
  locale: Locale;
  activeTab: AppTab;
  pluginPages: PluginPageDescriptor[];
  onTabChange: (tab: AppTab) => void;
};

const props = defineProps<NavigationRailProps>();

type NavigationTab = {
  id: AppTab;
  tooltip: string;
  icon?: Component | string;
  imageSrc?: string;
};

const builtinTabs = computed<NavigationTab[]>(() => [
  { id: 'feed', tooltip: t(props.locale, 'tabFeed'), icon: IconChat },
  { id: 'points', tooltip: t(props.locale, 'tabPoints'), icon: IconCoins },
  { id: 'analytics', tooltip: t(props.locale, 'tabAnalytics'), icon: IconBarChart },
  { id: 'connect', tooltip: t(props.locale, 'tabConnect'), icon: IconRadio },
  { id: 'behavior', tooltip: t(props.locale, 'tabBehavior'), icon: IconEdgeNet },
  { id: 'plugins', tooltip: t(props.locale, 'tabPlugins'), icon: IconPlugins },
  { id: 'widgets', tooltip: t(props.locale, 'tabWidgets'), imageSrc: widgetsOverlayGraphic },
  { id: 'settings', tooltip: t(props.locale, 'tabSettings'), icon: IconSettings },
]);

/**
 * Plugin page tabs appended after the builtins. Icons pass through the
 * registry allowlist (unknown manifest names fall back to the plugin glyph),
 * and labels are manifest data rendered as tooltip text only.
 *
 * Connection-only pages (a `connection` section plus optional intro text)
 * are hidden: the Connections tab renders the same card inline, so each
 * server-style plugin doesn't mint a duplicate minimal tab. Their page
 * icon is reused on the inline card instead.
 */
const pluginTabs = computed<NavigationTab[]>(() => props.pluginPages.filter((page) => !isConnectionOnlyPage(page)).map((page) => ({
  id: pluginNavId(page.pluginId, page.id),
  tooltip: i18nText(props.locale, page.title),
  icon: readIconName(page.icon) ?? 'plugin',
})));

const navTabs = computed<NavigationTab[]>(() => [...builtinTabs.value, ...pluginTabs.value]);
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
        <img
          v-if="tab.imageSrc"
          class="nav-tab-btn__image"
          :src="tab.imageSrc"
          alt=""
          aria-hidden="true"
        />
        <Icon v-else-if="typeof tab.icon === 'string'" :name="tab.icon" />
        <component v-else :is="tab.icon" />
      </button>
    </Tooltip>
  </nav>
</template>
