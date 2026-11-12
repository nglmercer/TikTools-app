<script setup lang="ts">
import BaseFileUpload from '@/shared/ui/file-upload/BaseFileUpload.vue';
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import OverlayLibraryPreview from '@/widgets/overlay-cloud/ui/OverlayLibraryPreview.vue';
const { t } = useI18n();
const open = ref(false);
const file = ref<File | null>(null);
</script>

<template>
  <section class="library-kit">
    <h2>{{ t('overlayLibrary.title') }}</h2>
    <p>{{ t('designKit.description') }}</p>
    <BaseButton @click="open = true">
      {{ t('overlayLibrary.browse') }}
    </BaseButton>
    <BaseDialog
      v-model:open="open"
      :title="t('overlayLibrary.title')"
      :width="800"
    >
      <div class="library-kit__grid">
        <OverlayLibraryPreview
          kind="follow"
          :label="t('overlayLibrary.preview', { name: t('overlays.kinds.follow.name') })"
        />
        <OverlayLibraryPreview
          kind="gift"
          :label="t('overlayLibrary.preview', { name: t('overlays.kinds.gift.name') })"
        />
        <BaseEmptyState :title="t('overlayLibrary.empty')" />
        <BaseEmptyState
          :title="t('overlayLibrary.loading')"
          aria-busy="true"
        />
        <BaseEmptyState :title="t('overlayLibrary.failed')" />
        <BaseButton
          variant="primary"
          disabled
        >
          {{ t('overlayLibrary.open') }}
        </BaseButton>
      </div>
      <BaseFileUpload
        v-model="file"
        :label="t('fileUpload.dropImage')"
        :hint="t('fileUpload.imageHint', { size: 512 })"
        accept="image/png,image/jpeg,image/webp"
        :max-bytes="512 * 1024"
      />
      <template #footer>
        <BaseButton @click="open = false">
          {{ t('common.close') }}
        </BaseButton>
      </template>
    </BaseDialog>
  </section>
</template>

<style scoped>
.library-kit { padding:var(--space-6); border:1px solid var(--color-border); background:var(--color-bg-panel); }
.library-kit h2 { margin:0; font-size:var(--font-size-lg); }
.library-kit p { color:var(--color-text-muted); }
.library-kit__grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(240px,1fr)); gap:var(--space-6); }
</style>
