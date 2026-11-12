<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { MAX_IMAGE_BYTES, useBundlesStore } from '@/entities/cloud-bundles/model/bundles-store.ts';
import BaseFileUpload from '@/shared/ui/file-upload/BaseFileUpload.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
const bundles = useBundlesStore();
const file = ref<File | null>(null);
async function upload(): Promise<void> {
  if (file.value && await bundles.upload(file.value)) file.value = null;
}
</script>

<template>
  <div class="cloud-image-upload">
    <BaseFileUpload
      v-model="file"
      :label="t('fileUpload.dropImage')"
      :hint="t('fileUpload.imageHint', { size: MAX_IMAGE_BYTES / 1024 })"
      accept="image/png,image/jpeg,image/webp,.png,.jpg,.jpeg,.webp"
      :max-bytes="MAX_IMAGE_BYTES"
      :disabled="bundles.working"
    />
    <div class="cloud-image-upload__footer">
      <span
        v-if="bundles.asset"
        role="status"
      >{{ t('cloudBundles.uploaded', { id: bundles.asset.id, width: bundles.asset.width, height: bundles.asset.height }) }}</span>
      <BaseButton
        variant="primary"
        :disabled="!file || bundles.working"
        @click="upload"
      >
        {{ bundles.working ? t('fileUpload.uploading') : t('cloudBundles.upload') }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
.cloud-image-upload { display:grid; gap:var(--space-5); }
.cloud-image-upload__footer { display:flex; flex-wrap:wrap; align-items:center; justify-content:flex-end; gap:var(--space-5); }
.cloud-image-upload__footer span { flex:1; overflow-wrap:anywhere; color:var(--color-text-muted); font-size:var(--font-size-sm); }
</style>
