<script setup lang="ts">
import { computed, markRaw, ref, shallowRef, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import { DEFAULT_PROFILE_ID, type ProfilePack } from '@/entities/rule-template/lib/rule-profiles.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseConfirmDialog from '@/shared/ui/dialog/BaseConfirmDialog.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{
  packs: readonly ProfilePack[];
  activeId: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  switch: [id: string];
  create: [name: string];
  delete: [id: string];
  export: [id: string];
  import: [];
}>();

const { t } = useI18n();
const id = useId();
const menuOpen = ref(false);
const menuAnchor = shallowRef<HTMLElement | null>(null);
const createOpen = ref(false);
const deleteOpen = ref(false);
const name = ref('');

const packName = (pack: ProfilePack): string => (pack.id === DEFAULT_PROFILE_ID ? t('automations.profile.default') : pack.name);
const options = computed(() =>
  props.packs.map((pack) => ({
    value: pack.id,
    label: `${packName(pack)} · ${t('automations.profile.counts', { events: pack.eventIds.length, actions: pack.actionIds.length })}`,
  })),
);
const active = computed(() => props.packs.find((pack) => pack.id === props.activeId) ?? null);
const menu = computed<MenuEntry[]>(() => [
  { id: 'create', label: t('automations.profile.create') },
  { id: 'import', label: t('automations.profile.import') },
  { id: 'export', label: t('automations.profile.export'), disabled: !active.value || active.value.id === DEFAULT_PROFILE_ID },
  { type: 'separator', id: 'separator' },
  { id: 'delete', label: t('automations.profile.delete'), disabled: !active.value || active.value.id === DEFAULT_PROFILE_ID },
]);

function openMenu(event: MouseEvent): void {
  menuAnchor.value = markRaw(event.currentTarget as HTMLElement);
  menuOpen.value = true;
}

function onMenu(action: MenuAction): void {
  if (props.disabled) return;
  if (action.id === 'create') {
    name.value = '';
    createOpen.value = true;
  } else if (action.id === 'import') emit('import');
  else if (action.id === 'export' && active.value) emit('export', active.value.id);
  else if (action.id === 'delete') deleteOpen.value = true;
}

function create(): void {
  if (props.disabled) return;
  const value = name.value.trim();
  if (!value) return;
  createOpen.value = false;
  emit('create', value);
}

function confirmDelete(): void {
  if (props.disabled) return;
  deleteOpen.value = false;
  if (active.value) emit('delete', active.value.id);
}
</script>

<template>
  <div class="profile-toolbar">
    <label
      class="profile-toolbar__label"
      :for="`${id}-profile`"
    >{{ t('automations.profile.label') }}</label>
    <BaseComboBox
      :id="`${id}-profile`"
      class="profile-toolbar__select"
      :model-value="activeId"
      :options="options"
      size="sm"
      :disabled="disabled"
      @update:model-value="$event && $event !== activeId && emit('switch', $event)"
    />
    <BaseIconButton
      :label="t('automations.profile.menu')"
      size="sm"
      aria-haspopup="menu"
      :disabled="disabled"
      @click="openMenu"
    >
      <IconMoreVertical />
    </BaseIconButton>
    <BaseMenu
      v-model:open="menuOpen"
      :items="menu"
      :anchor="menuAnchor"
      @select="onMenu"
    />
    <BaseDialog
      v-model:open="createOpen"
      :title="t('automations.profile.createTitle')"
      :width="400"
    >
      <form @submit.prevent="create">
        <BaseFormGrid>
          <BaseFormRow
            :for="`${id}-name`"
            :label="t('automations.profile.createName')"
          >
            <BaseTextField
              :id="`${id}-name`"
              v-model="name"
              maxlength="80"
            />
          </BaseFormRow>
        </BaseFormGrid>
      </form>
      <template #footer>
        <BaseButton
          variant="primary"
          :disabled="disabled || !name.trim()"
          @click="create"
        >
          {{ t('automations.profile.createConfirm') }}
        </BaseButton>
        <BaseButton @click="createOpen = false">
          {{ t('common.cancel') }}
        </BaseButton>
      </template>
    </BaseDialog>
    <BaseConfirmDialog
      v-model:open="deleteOpen"
      :title="t('automations.profile.deleteTitle')"
      :message="
        active
          ? t('automations.profile.deleteMessage', { name: packName(active), events: active.eventIds.length, actions: active.actionIds.length })
          : ''
      "
      :confirm-label="t('automations.profile.deleteConfirm')"
      danger
      @confirm="confirmDelete"
    />
  </div>
</template>

<style scoped>
.profile-toolbar {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
}

.profile-toolbar__label {
  color: var(--color-text-muted);
}

.profile-toolbar__select {
  /* Wide enough for "name · N events · N actions" without truncation. */
  width: clamp(240px, 28vw, 380px);
}

/* Phones: the picker takes its own full-width row. */
@media (width < 600px) {
  .profile-toolbar {
    display: flex;
    flex: 1 1 100%;
    min-width: 0;
  }

  .profile-toolbar__select {
    flex: 1;
    width: auto;
    min-width: 0;
  }
}
</style>
