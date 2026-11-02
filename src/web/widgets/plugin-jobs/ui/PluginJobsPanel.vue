<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JobSnapshot } from '@/entities/plugin-package/api/package-types.ts';
import { installErrorKey } from '@/entities/plugin-package/lib/install-error.ts';
import { isActiveJob, jobPercent } from '@/entities/plugin-package/lib/package-format.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';

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

function status(job: JobSnapshot): string {
  if (job.error) {
    const key = installErrorKey(job.error.code);
    return te(key) ? t(key) : job.error.message;
  }
  if (job.status === 'succeeded') return t('packages.jobs.done');
  const percent = jobPercent(job);
  const stage = t(`packages.jobs.stage.${job.stage}`);
  return percent === null ? stage : `${stage} · ${percent}%`;
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
      <span class="plugin-jobs__label">{{ label(job) }}</span>
      <span class="plugin-jobs__status">{{ status(job) }}</span>
      <progress
        v-if="isActiveJob(job)"
        class="plugin-jobs__bar"
        :value="jobPercent(job) ?? undefined"
        max="100"
      />
      <BaseButton
        v-if="isActiveJob(job)"
        size="sm"
        @click="packages.cancelJob(job.id)"
      >
        {{ t('common.cancel') }}
      </BaseButton>
      <BaseButton
        v-else
        size="sm"
        variant="ghost"
        @click="dismiss(job.id)"
      >
        {{ t('common.close') }}
      </BaseButton>
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
  padding: var(--space-2) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
}

.plugin-jobs__item--succeeded {
  border-color: var(--color-success);
}

.plugin-jobs__item--failed {
  border-color: var(--color-danger);
}

.plugin-jobs__label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plugin-jobs__status {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.plugin-jobs__bar {
  width: 120px;
  height: 6px;
}
</style>
