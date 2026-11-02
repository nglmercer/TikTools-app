<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PackagePreview } from '@/entities/plugin-package/api/package-types.ts';
import { installErrorKey } from '@/entities/plugin-package/lib/install-error.ts';
import { formatBytes, originDetail, originLabelKey, shortDigest } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';

const { t, te } = useI18n();
const packages = usePackagesStore();
const sideloadAccepted = ref(false);

const review = computed(() => packages.review);
const preview = computed<PackagePreview | null>(() => review.value.preview);
const open = computed({
  get: () => review.value.open,
  set: (value: boolean) => {
    if (!value) packages.closeReview();
  },
});

watch(
  () => preview.value?.consentToken,
  () => {
    sideloadAccepted.value = false;
  },
);

const title = computed(() =>
  preview.value
    ? t(`packages.review.title.${preview.value.change}`, { name: preview.value.name })
    : t('packages.review.loadingTitle'),
);

const confirmLabel = computed(() =>
  preview.value ? t(`packages.review.confirm.${preview.value.change}`) : t('packages.review.confirm.install'),
);

const canConfirm = computed(
  () =>
    !!preview.value &&
    !preview.value.incompatibility &&
    !review.value.submitting &&
    (!preview.value.requiresSideloadConsent || sideloadAccepted.value),
);

const accessChanges = computed(() => {
  const diff = preview.value?.permissionDiff;
  if (!diff || preview.value?.change === 'install') return [];
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

function errorText(): string {
  const error = review.value.error;
  if (!error) return '';
  return te(installErrorKey(error.code)) ? t(installErrorKey(error.code)) : t('packages.errors.unsupported');
}

function incompatibilityText(): string {
  const value = preview.value?.incompatibility;
  if (!value) return '';
  return t(`packages.incompatible.${value.kind}`, {
    required: value.required ?? '',
    host: value.host ?? '',
    runtime: value.runtime ?? '',
    version: value.version ?? '',
  });
}

async function confirm(): Promise<void> {
  if (!canConfirm.value) return;
  await packages.confirmReview();
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="title"
    :width="560"
    :closable="!review.submitting"
  >
    <p
      v-if="review.loading"
      class="install-dialog__muted"
      role="status"
    >
      {{ t('packages.review.loading') }}
    </p>
    <BaseAlert
      v-if="review.error"
      :title="errorText()"
    >
      <p class="install-dialog__detail">
        {{ review.error.message }}
      </p>
    </BaseAlert>
    <div
      v-if="preview"
      class="install-dialog"
    >
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

      <section
        v-if="preview.changelog"
        class="install-dialog__section"
      >
        <h3 class="install-dialog__heading">
          {{ t('packages.review.changelog') }}
        </h3>
        <div class="install-dialog__changelog">
          <BaseMarkdown :source="preview.changelog" />
        </div>
      </section>

      <BaseCheckbox
        v-if="preview.requiresSideloadConsent && !preview.incompatibility"
        v-model="sideloadAccepted"
      >
        {{ t('packages.review.sideload') }}
      </BaseCheckbox>
    </div>

    <template #footer>
      <BaseButton
        variant="primary"
        :disabled="!canConfirm"
        @click="confirm"
      >
        {{ review.submitting ? t('packages.review.working') : confirmLabel }}
      </BaseButton>
      <BaseButton
        :disabled="review.submitting"
        @click="packages.closeReview()"
      >
        {{ t('common.cancel') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.install-dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.install-dialog__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
}

.install-dialog__name {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
}

.install-dialog__muted,
.install-dialog__detail {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.install-dialog__mono {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-normal);
}

.install-dialog__badge {
  flex: none;
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  line-height: 22px;
}

.install-dialog__badge--verified {
  border-color: var(--color-success);
  color: var(--color-success);
}

.install-dialog__badge--unsigned,
.install-dialog__badge--untrusted {
  border-color: var(--color-warning);
  color: var(--color-warning);
}

.install-dialog__description {
  margin: 0;
  line-height: var(--line-height-normal);
}

.install-dialog__facts {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--space-2) var(--space-6);
  margin: 0;
}

.install-dialog__facts dt {
  color: var(--color-text-muted);
}

.install-dialog__facts dd {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
  overflow-wrap: anywhere;
}

.install-dialog__section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.install-dialog__heading {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.install-dialog__chips,
.install-dialog__changes {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.install-dialog__chip {
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  line-height: 20px;
}

.install-dialog__added {
  color: var(--color-warning);
}

.install-dialog__removed {
  color: var(--color-text-muted);
}

.install-dialog__changelog {
  max-height: 160px;
  padding: var(--space-3);
  overflow: auto;
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
}
</style>
