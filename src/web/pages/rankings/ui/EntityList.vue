<script setup lang="ts">
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';

export interface EntityItem { id: string; title: string; subtitle: string }

defineProps<{ title: string; createLabel: string; emptyLabel: string; items: EntityItem[]; selected: string | null }>();
const emit = defineEmits<{ select: [id: string]; create: [] }>();
</script>

<template>
  <BasePanel
    :title="title"
    flush
  >
    <template #actions>
      <BaseButton
        size="sm"
        variant="primary"
        @click="emit('create')"
      >
        {{ createLabel }}
      </BaseButton>
    </template>
    <ul
      v-if="items.length"
      class="entity-list"
    >
      <li
        v-for="item in items"
        :key="item.id"
      >
        <button
          type="button"
          class="entity-list__item"
          :class="{ 'entity-list__item--active': item.id === selected }"
          :data-item="item.id"
          @click="emit('select', item.id)"
        >
          <strong>{{ item.title }}</strong>
          <span>{{ item.subtitle }}</span>
        </button>
      </li>
    </ul>
    <p
      v-else
      class="entity-list__empty"
    >
      {{ emptyLabel }}
    </p>
  </BasePanel>
</template>

<style scoped>
.entity-list { list-style: none; margin: 0; padding: var(--space-1); display: grid; gap: 2px; }
.entity-list__item { display: grid; gap: 2px; width: 100%; text-align: left; padding: var(--space-2) var(--space-3); color: inherit; background: transparent; border: 1px solid transparent; border-radius: var(--radius-sm); cursor: pointer; }
.entity-list__item:hover { background: var(--color-bg-control); }
.entity-list__item--active { background: var(--color-bg-control); border-color: var(--color-accent, #3b82f6); }
.entity-list__item span { font-size: 12px; color: var(--color-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.entity-list__empty { margin: 0; padding: var(--space-3); color: var(--color-text-muted); }
</style>
