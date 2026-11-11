<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JobSnapshot } from '@/entities/plugin-package/api/package-types.ts';
import { installErrorKey } from '@/entities/plugin-package/lib/install-error.ts';
import { isActiveJob, jobPercent } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconStop from '@/shared/ui/icon/icons/IconStop.vue';

const RECENT_MS = 2 * 60 * 1000;

const { t, te } = useI18n();
const packages = usePackagesStore();
const dismissed = ref(new Set<string>());
const mountedAt = Date.now();

const visible = computed(() =>
  packages.jobs.filter((job) => {
    if (dismissed.value.has(job.id)) return false;
    if (isActiveJob(job)) return true;
    const finishedRecently = job.updatedAt * 1000 >= Math.max(mountedAt, Date.now() - RECENT_MS);
    return finishedRecently && job.error?.code !== 'consent-required';
  }),
);

function label(job: JobSnapshot): string {
  return t(`packages.jobs.kind.${job.kind}`, {
    id: job.pluginId ?? job.spec ?? '',
    version: job.toVersion ?? '',
  });
}

/** What the strip says after the name: the percentage while running, then the outcome. */
function status(job: JobSnapshot): string {
  if (job.error) {
    const key = installErrorKey(job.error.code);
    return te(key) ? t(key) : job.error.message;
  }
  if (job.status === 'succeeded') return t('packages.jobs.done');
  const percent = jobPercent(job);
  return percent === null ? '' : `${percent}%`;
}

function dismiss(id: string): void {
  dismissed.value = new Set([...dismissed.value, id]);
}
</script>

<template>
  <ul
    v-if="visible.length"
    class="plugin-jobs"
    :aria-label="t('packages.jobs.title')"
  >
    <li
      v-for="job in visible"
      :key="job.id"
      class="plugin-jobs__item"
      :class="`plugin-jobs__item--${job.status}`"
      :role="job.error ? 'alert' : 'status'"
    >
      <IconDownload class="plugin-jobs__icon" />
      <span class="plugin-jobs__label">{{ label(job) }}</span>
      <progress
        v-if="isActiveJob(job)"
        class="plugin-jobs__bar"
        :value="jobPercent(job) ?? undefined"
        max="100"
        :aria-label="t(`packages.jobs.stage.${job.stage}`)"
      />
      <span
        v-if="status(job)"
        class="plugin-jobs__status"
      >{{ status(job) }}</span>
      <BaseIconButton
        v-if="isActiveJob(job)"
        :label="t('common.cancel')"
        size="sm"
        @click="packages.cancelJob(job.id)"
      >
        <IconStop />
      </BaseIconButton>
      <BaseIconButton
        v-else
        :label="t('common.close')"
        size="sm"
        @click="dismiss(job.id)"
      >
        <IconClose />
      </BaseIconButton>
    </li>
  </ul>
</template>

<style scoped>
.plugin-jobs {
  display: flex;
  flex: none;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.plugin-jobs__item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: var(--control-height-lg);
  padding: 0 var(--space-2) 0 var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.plugin-jobs__item--succeeded {
  border-color: var(--color-success);
}

.plugin-jobs__item--failed {
  border-color: var(--color-danger);
}

.plugin-jobs__icon {
  color: var(--color-accent);
}

.plugin-jobs__label {
  flex: none;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-jobs__status {
  flex: none;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.plugin-jobs__item--failed .plugin-jobs__status {
  flex: 1;
  min-width: 0;
  font-family: inherit;
  overflow-wrap: anywhere;
}

.plugin-jobs__bar {
  flex: 1;
  min-width: 60px;
  height: 6px;
}

/* Without a bar the name takes the width, so the outcome and the button stay at the end. */
.plugin-jobs__item:not(:has(.plugin-jobs__bar)) .plugin-jobs__label {
  flex: 1;
}
</style>
