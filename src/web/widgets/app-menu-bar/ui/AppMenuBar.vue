<script setup lang="ts">
import { computed, markRaw, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import { isConnectionBusy } from '@/entities/connection/lib/connection-status.ts';
import { supportedLocales, type Locale } from '@/shared/i18n/i18n.ts';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconMenu from '@/shared/ui/icon/icons/IconMenu.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseMenuBar from '@/shared/ui/menu/BaseMenuBar.vue';
import type { MenuAction, MenuBarMenu } from '@/shared/ui/menu/menu-types.ts';
import type { ConnectionStatus } from '@/types.ts';

interface MenuPage {
  id: string;
  label: string;
  icon: Component;
}

const props = defineProps<{
  status: ConnectionStatus;
  locale: Locale;
  pages: readonly MenuPage[];
  sidebarExpanded: boolean;
}>();

const emit = defineEmits<{
  connect: [];
  disconnect: [];
  reconnect: [];
  navigate: [itemId: string];
  'change-locale': [locale: Locale];
  'toggle-sidebar': [];
}>();

const { t } = useI18n();

const menus = computed<MenuBarMenu[]>(() => [
  {
    id: 'file',
    label: t('menu.file'),
    items: [
      { id: 'connect', label: t('connection.connect'), disabled: props.status === 'connected' || isConnectionBusy(props.status) },
      { id: 'reconnect', label: t('connection.reconnect'), disabled: props.status === 'idle' || isConnectionBusy(props.status) },
      { id: 'disconnect', label: t('connection.disconnect'), disabled: props.status !== 'connected' && !isConnectionBusy(props.status) },
      ...(window.ipc && window.__tiktools_remote_client_available__ ? [{ id: 'remote-controller', label: t('menu.remoteController') }] : []),
      { type: 'separator', id: 'file-separator' },
      { id: 'navigate:settings', label: t('navigation.settings'), icon: markRaw(IconGear) },
    ],
  },
  {
    id: 'view',
    label: t('menu.view'),
    items: [
      ...props.pages.map((page) => ({ id: `navigate:${page.id}`, label: page.label, icon: markRaw(page.icon) })),
      { type: 'separator', id: 'view-separator' },
      {
        id: 'language',
        label: t('menu.language'),
        icon: markRaw(IconGlobe),
        children: supportedLocales.map((locale) => ({
          id: `locale:${locale}`,
          label: t(`menu.languages.${locale}`),
          checked: locale === props.locale,
        })),
      },
    ],
  },
]);

function onSelect(action: MenuAction): void {
  const separator = action.id.indexOf(':');
  const kind = separator < 0 ? action.id : action.id.slice(0, separator);
  const value = separator < 0 ? undefined : action.id.slice(separator + 1);
  if (kind === 'connect') emit('connect');
  else if (kind === 'reconnect') emit('reconnect');
  else if (kind === 'disconnect') emit('disconnect');
  else if (kind === 'remote-controller') window.ipc?.postMessage(JSON.stringify({ type: 'open-controller' }));
  else if (kind === 'navigate' && value) emit('navigate', value);
  else if (kind === 'locale' && value && supportedLocales.includes(value as Locale)) emit('change-locale', value as Locale);
}
</script>

<template>
  <header class="app-menu-bar">
    <BaseIconButton
      :label="t('navigation.toggleSidebar')"
      size="sm"
      :pressed="sidebarExpanded"
      :aria-expanded="sidebarExpanded"
      aria-controls="app-sidebar"
      @click="emit('toggle-sidebar')"
    >
      <IconMenu />
    </BaseIconButton>
    <BaseMenuBar
      :menus="menus"
      @select="onSelect"
    />
    <span class="app-menu-bar__title">TikTools</span>
  </header>
</template>

<style scoped>
.app-menu-bar {
  display: flex;
  flex: none;
  align-items: center;
  gap: var(--space-4);
  height: var(--chrome-height);
  padding: 0 var(--space-2);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
  font-family: var(--font-family);
}

.app-menu-bar__title {
  margin-left: auto;
  padding-right: var(--space-4);
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}
</style>
