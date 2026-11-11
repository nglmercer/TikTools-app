<script setup lang="ts">
import { failureText } from '@/shared/api/cloud-failure.ts';
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useBundlesStore } from '@/entities/cloud-bundles/model/bundles-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconInfo from '@/shared/ui/icon/icons/IconInfo.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

const { t, te } = useI18n();
const bundles = useBundlesStore();
const packageInput = ref('');
const file = ref<File | null>(null);

function statusLabel(value: string): string {
  const key = `cloudBundles.status.${value}`;
  return te(key) ? t(key) : value;
}

function onFile(event: Event): void {
  file.value = (event.target as HTMLInputElement).files?.[0] ?? null;
}

async function upload(): Promise<void> {
  if (file.value) await bundles.upload(file.value);
}

function journalStatus(value: string): string {
  return t(`cloudBundles.journal.${value}`);
}

async function apply(): Promise<void> {
  await bundles.applyPackage();
}

onMounted(() => {
  if (bundles.available !== false) void bundles.loadJournal();
});

async function check(): Promise<void> {
  const id = packageInput.value.trim();
  if (id.length > 0) await bundles.checkPackage(id);
}
</script>

<template>
  <section class="cloud-bundles">
    <h3 class="cloud-bundles__title">
      {{ t('cloudBundles.title') }}
      <!-- The explanation stays one hover away instead of a paragraph under the heading. -->
      <span
        v-tooltip="t('cloudBundles.hint')"
        class="cloud-bundles__info"
        role="img"
        :aria-label="t('cloudBundles.hint')"
      >
        <IconInfo :size="14" />
      </span>
    </h3>

    <BaseAlert
      v-if="bundles.error"
      tone="error"
      :title="bundles.error === 'size' ? t('cloudBundles.tooLarge') : failureText(t, bundles.errorDetail, bundles.error)"
      compact
    />

    <div class="cloud-bundles__block">
      <h4>{{ t('cloudBundles.assets') }}</h4>
      <div class="cloud-bundles__row">
        <input
          type="file"
          accept="image/png,image/jpeg,image/webp"
          :aria-label="t('cloudBundles.chooseImage')"
          @change="onFile"
        >
        <BaseButton
          :disabled="!file || bundles.working"
          @click="upload"
        >
          {{ t('cloudBundles.upload') }}
        </BaseButton>
      </div>
      <p
        v-if="bundles.asset"
        class="cloud-bundles__note"
      >
        {{ t('cloudBundles.uploaded', { id: bundles.asset.id, width: bundles.asset.width, height: bundles.asset.height }) }}
      </p>
    </div>

    <div class="cloud-bundles__block">
      <h4>{{ t('cloudBundles.preflight') }}</h4>
      <form
        class="cloud-bundles__row"
        @submit.prevent="check"
      >
        <BaseTextField
          v-model="packageInput"
          class="cloud-bundles__grow"
          :placeholder="t('cloudBundles.packageId')"
          :aria-label="t('cloudBundles.packageId')"
        />
        <BaseButton
          type="submit"
          :disabled="packageInput.trim().length === 0 || bundles.working"
        >
          {{ t('cloudBundles.check') }}
        </BaseButton>
      </form>
      <template v-if="bundles.preflight">
        <p class="cloud-bundles__note">
          {{ bundles.preflight.canApply ? t('cloudBundles.canApply') : t('cloudBundles.blocked') }}
        </p>
        <ul class="cloud-bundles__list">
          <li
            v-for="item in bundles.preflight.items"
            :key="`${item.kind}:${item.id}`"
            class="cloud-bundles__row"
          >
            <span class="cloud-bundles__grow">
              {{ t(`cloudBundles.kinds.${item.kind}`) }} · {{ item.id }}
              <template v-if="item.version"> · {{ item.version }}</template>
              <template v-if="item.revision !== null"> · r{{ item.revision }}</template>
            </span>
            <span>{{ statusLabel(item.status) }}</span>
          </li>
        </ul>
        <div class="cloud-bundles__row">
          <BaseButton
            v-if="bundles.preflight.canApply && !bundles.applying"
            variant="primary"
            @click="apply"
          >
            {{ t('cloudBundles.apply') }}
          </BaseButton>
          <BaseButton
            v-if="bundles.applying"
            variant="ghost"
            @click="bundles.cancelApply()"
          >
            {{ t('cloudBundles.cancelApply') }}
          </BaseButton>
        </div>
      </template>
    </div>

    <div
      v-if="bundles.journal"
      class="cloud-bundles__block"
    >
      <h4>{{ t('cloudBundles.journalTitle') }}</h4>
      <ol class="cloud-bundles__list">
        <li
          v-for="(entry, index) in bundles.journal.entries"
          :key="index"
          class="cloud-bundles__row"
        >
          <span class="cloud-bundles__grow">
            {{ t(`cloudBundles.kinds.${entry.kind}`) }} · {{ entry.id }}
            <template v-if="entry.message">
              <br>
              <small>{{ entry.message }}</small>
            </template>
          </span>
          <span>{{ journalStatus(entry.status) }}</span>
        </li>
      </ol>
      <BaseAlert
        v-if="bundles.applyError"
        tone="error"
        :title="bundles.applyError"
        compact
      />
      <BaseButton
        v-if="!bundles.applying"
        variant="ghost"
        @click="bundles.dismissJournal()"
      >
        {{ t('cloudBundles.dismissJournal') }}
      </BaseButton>
    </div>
  </section>
</template>

<style scoped>
.cloud-bundles { display:grid; gap:var(--space-3); min-width:0; }
.cloud-bundles__title, .cloud-bundles__block h4 { margin:0; }
.cloud-bundles__info { display:inline-flex; margin-left:var(--space-2); vertical-align:middle; color:var(--color-text-muted); font-weight:normal; }
.cloud-bundles__note { margin:0; color:var(--color-text-muted); }
.cloud-bundles__row { display:flex; flex-wrap:wrap; gap:var(--space-2); align-items:center; min-width:0; }
.cloud-bundles__grow { flex:1 1 200px; min-width:0; overflow-wrap:anywhere; }
.cloud-bundles__block { display:grid; gap:var(--space-2); padding:var(--space-3); border-radius:var(--radius-md); border:1px solid var(--color-border); min-width:0; }
.cloud-bundles__list { list-style:none; margin:0; padding:0; display:grid; gap:var(--space-2); }
</style>
