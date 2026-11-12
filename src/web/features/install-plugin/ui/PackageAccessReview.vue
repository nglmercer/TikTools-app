<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PackagePreview } from '@/entities/plugin-package/api/package-types.ts';

const props = defineProps<{ preview: PackagePreview }>();
const { t, te } = useI18n();

const accessChanges = computed(() => {
  const diff = props.preview.permissionDiff;
  if (!diff || props.preview.change === 'install') return [];
  return [
    ...diff.addedPermissions.map((value) => ({ value, added: true })),
    ...diff.addedCapabilities.map((value) => ({ value, added: true })),
    ...diff.removedPermissions.map((value) => ({ value, added: false })),
    ...diff.removedCapabilities.map((value) => ({ value, added: false })),
  ];
});

function accessLabel(value: string): string {
  const key = `packages.access.${value.replaceAll('.', '_')}`;
  return te(key) ? t(key) : value;
}
</script>

<template>
  <section class="install-dialog__section">
    <h3 class="install-dialog__heading">
      {{ t('packages.review.access') }}
    </h3>
    <p
      v-if="!preview.permissions.length && !preview.capabilities.length"
      class="install-dialog__muted"
    >
      {{ t('packages.review.noAccess') }}
    </p>
    <ul
      v-else
      class="install-dialog__chips"
    >
      <li
        v-for="value in [...preview.permissions, ...preview.capabilities]"
        :key="value"
        class="install-dialog__chip"
      >
        {{ accessLabel(value) }}
      </li>
    </ul>
    <ul
      v-if="accessChanges.length"
      class="install-dialog__changes"
    >
      <li
        v-for="change in accessChanges"
        :key="`${change.added}-${change.value}`"
        :class="change.added ? 'install-dialog__added' : 'install-dialog__removed'"
      >
        {{ change.added ? '+' : '−' }} {{ accessLabel(change.value) }}
      </li>
    </ul>
    <p class="install-dialog__muted">
      {{ t(`packages.security.${preview.securityModel}Hint`) }}
    </p>
  </section>
</template>

<style scoped src="./install-plugin-dialog.css"></style>
