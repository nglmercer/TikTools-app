<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';

/** Technical Cloud session details for Settings › Advanced. Nonsecret only. */
const { t } = useI18n();
const account = useAccountStore();

const rows = computed(() => {
  const status = account.status;
  if (!status) return [];
  return [
    { key: t('account.details.origin'), value: status.origin ?? t('account.details.disabled') },
    { key: t('account.details.state'), value: status.state },
    ...(status.accountId ? [{ key: t('account.details.accountId'), value: status.accountId }] : []),
    {
      key: t('account.details.storage'),
      value: t(status.persistence === 'keychain' ? 'account.storedInKeychain' : 'account.storedInMemory'),
    },
    ...(account.error ? [{ key: t('account.details.lastError'), value: account.error }] : []),
  ];
});

onMounted(() => {
  if (!account.status) void account.refresh();
});
</script>

<template>
  <BaseGroupBox :title="t('account.details.title')">
    <dl class="cloud-diagnostics">
      <template
        v-for="row in rows"
        :key="row.key"
      >
        <dt>{{ row.key }}</dt>
        <dd>{{ row.value }}</dd>
      </template>
    </dl>
  </BaseGroupBox>
</template>

<style scoped>
.cloud-diagnostics {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-2) var(--space-4);
  margin: 0;
  font-size: var(--font-size-sm);
}

.cloud-diagnostics dt {
  color: var(--color-text-muted);
}

.cloud-diagnostics dd {
  margin: 0;
  overflow-wrap: anywhere;
  font-family: var(--font-family-mono);
}
</style>
