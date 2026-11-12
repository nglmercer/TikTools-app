<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PackagePreview } from '@/entities/plugin-package/api/package-types.ts';
import { installErrorKey } from '@/entities/plugin-package/lib/install-error.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseMarkdown from '@/shared/ui/markdown/BaseMarkdown.vue';
import PackagePreviewSummary from './PackagePreviewSummary.vue';
import PackageAccessReview from './PackageAccessReview.vue';

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

function errorText(): string {
  const error = review.value.error;
  if (!error) return '';
  return te(installErrorKey(error.code)) ? t(installErrorKey(error.code)) : t('packages.errors.unsupported');
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
      <PackagePreviewSummary :preview="preview" />
      <PackageAccessReview :preview="preview" />
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

<style scoped src="./install-plugin-dialog.css"></style>
