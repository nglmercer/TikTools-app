<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PackagePreview } from '@/entities/plugin-package/api/package-types.ts';
import { formatBytes, originDetail, originLabelKey, shortDigest } from '@/entities/plugin-package/lib/package-format.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';

const props = defineProps<{ preview: PackagePreview }>();
const { t } = useI18n();

function incompatibilityText(): string {
  const value = props.preview.incompatibility;
  if (!value) return '';
  return t(`packages.incompatible.${value.kind}`, {
    required: value.required ?? '',
    host: value.host ?? '',
    runtime: value.runtime ?? '',
    version: value.version ?? '',
  });
}
</script>

<template>
  <header class="install-dialog__head">
    <div>
      <p class="install-dialog__name">
        {{ preview.name }}
        <span class="install-dialog__mono">{{ preview.version }}</span>
      </p>
      <p class="install-dialog__muted">
        {{ preview.id }}
        <template v-if="preview.installedVersion">
          · {{ t('packages.review.installed', { version: preview.installedVersion }) }}
        </template>
      </p>
    </div>
    <span
      class="install-dialog__badge"
      :class="`install-dialog__badge--${preview.signature.status}`"
    >
      {{ t(`packages.signature.${preview.signature.status}`, { publisher: preview.signature.publisher ?? '' }) }}
    </span>
  </header>

  <BaseAlert
    v-if="preview.incompatibility"
    :title="t('packages.review.incompatibleTitle')"
  >
    <p class="install-dialog__detail">
      {{ incompatibilityText() }}
    </p>
  </BaseAlert>
  <BaseAlert
    v-if="preview.change === 'downgrade'"
    tone="warning"
    :title="t('packages.review.downgrade')"
    compact
  />

  <p
    v-if="preview.description"
    class="install-dialog__description"
  >
    {{ preview.description }}
  </p>

  <dl class="install-dialog__facts">
    <dt>{{ t('packages.review.source') }}</dt>
    <dd>
      {{ t(originLabelKey(preview.origin)) }}
      <span
        v-if="originDetail(preview.origin)"
        class="install-dialog__mono"
      >{{ originDetail(preview.origin) }}</span>
    </dd>
    <dt>{{ t('packages.review.publisher') }}</dt>
    <dd>{{ preview.metadata.publisher ?? preview.signature.publisher ?? t('packages.review.unknown') }}</dd>
    <dt>{{ t('packages.review.runtime') }}</dt>
    <dd>
      {{ preview.runtime }} · {{ t(`packages.security.${preview.securityModel}`) }}
    </dd>
    <dt>{{ t('packages.review.size') }}</dt>
    <dd>{{ formatBytes(preview.size) }}</dd>
    <dt v-if="preview.metadata.license">
      {{ t('packages.review.license') }}
    </dt>
    <dd v-if="preview.metadata.license">
      {{ preview.metadata.license }}
    </dd>
    <dt>{{ t('packages.review.digest') }}</dt>
    <dd class="install-dialog__mono">
      {{ shortDigest(preview.digest) }}
    </dd>
  </dl>
</template>

<style scoped src="./install-plugin-dialog.css"></style>
