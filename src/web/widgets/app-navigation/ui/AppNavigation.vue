<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { NavigationSection } from '../model/navigation-types.ts';

withDefaults(
  defineProps<{
    sections: readonly NavigationSection[];
    collapsed?: boolean;
    /** Section suggested by the app state, e.g. the feed after a connection succeeds. */
    suggested?: string;
  }>(),
  { collapsed: false, suggested: '' },
);

const { t } = useI18n();
</script>

<template>
  <nav
    class="app-navigation"
    :class="{ 'app-navigation--collapsed': collapsed }"
    :aria-label="t('navigation.label')"
  >
    <section
      v-for="section in sections"
      :key="section.id"
      class="app-navigation__section"
      :aria-labelledby="`navigation-${section.id}`"
    >
      <h2
        :id="`navigation-${section.id}`"
        class="app-navigation__heading"
      >
        {{ section.label }}
      </h2>
      <ul class="app-navigation__list">
        <li
          v-for="item in section.items"
          :key="item.id"
        >
          <RouterLink
            class="app-navigation__link"
            :class="{ 'app-navigation__link--suggested': item.id === suggested }"
            exact-active-class="app-navigation__link--active"
            :to="item.to"
            :aria-label="collapsed ? item.label : undefined"
            :title="collapsed ? item.label : undefined"
          >
            <component
              :is="item.icon"
              class="app-navigation__icon"
            />
            <span
              v-if="!collapsed"
              class="app-navigation__label"
            >{{ item.label }}</span>
          </RouterLink>
        </li>
      </ul>
    </section>
  </nav>
</template>

<style scoped>
.app-navigation {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  height: 100%;
  padding: var(--space-3) var(--space-2);
  overflow-y: auto;
  background: var(--color-bg-panel);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.app-navigation__heading {
  margin: 0;
  padding: var(--space-2) var(--space-3);
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.app-navigation__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.app-navigation__link {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: var(--control-height-lg);
  padding: 0 var(--space-4);
  border: var(--border-width) solid transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  text-decoration: none;
  outline: none;
}

.app-navigation__link:hover {
  border-color: var(--color-border-selected);
  background: var(--color-bg-selected);
  color: var(--color-text);
}

.app-navigation__link:active {
  background: var(--color-accent);
  color: var(--color-text-on-accent);
}

.app-navigation__link:focus-visible {
  border-color: var(--color-border-focus);
}

.app-navigation__link--active,
.app-navigation__link--active:hover {
  border-color: var(--color-border-selected);
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

/* A connected creator has events to read: point the reader at the feed. */
.app-navigation__link--suggested:not(.app-navigation__link--active) {
  border-color: var(--color-accent);
  color: var(--color-text);
  box-shadow: inset 3px 0 0 var(--color-accent);
}

.app-navigation__link--suggested:not(.app-navigation__link--active)::after {
  content: '';
  width: 6px;
  height: 6px;
  margin-left: auto;
  border-radius: 50%;
  background: var(--color-accent);
}

.app-navigation__icon {
  flex: none;
}

.app-navigation__label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-navigation--collapsed {
  padding: var(--space-2);
}

.app-navigation--collapsed .app-navigation__heading {
  display: none;
}

.app-navigation--collapsed .app-navigation__link {
  justify-content: center;
  padding: 0;
}
</style>
