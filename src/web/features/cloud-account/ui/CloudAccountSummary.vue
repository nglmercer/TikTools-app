<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconLogOut from '@/shared/ui/icon/icons/IconLogOut.vue';

/** Who is signed in, the plan they are on, and how to sign out, in one compact card. */
const { t } = useI18n();
const account = useAccountStore();

const name = computed(() => account.status?.displayName ?? account.status?.email ?? account.status?.accountId ?? '');
/** Secondary line: the email, unless it is already the primary name. */
const handle = computed(() => {
  const email = account.status?.email ?? null;
  return email && email !== name.value ? email : null;
});
const initials = computed(() => {
  const letters = name.value.replace(/[^\p{L}\p{N} ]/gu, ' ').trim().split(/\s+/).filter(Boolean);
  const text = letters.length > 1 ? `${letters[0]![0]}${letters[1]![0]}` : (letters[0] ?? '?').slice(0, 2);
  return text.toLocaleUpperCase();
});
const planName = computed(() => {
  const plan = account.plan?.plan ?? '';
  return plan ? plan.charAt(0).toUpperCase() + plan.slice(1) : '';
});
</script>

<template>
  <section
    class="account-summary"
    :aria-label="t('account.group')"
  >
    <span
      class="account-summary__avatar"
      aria-hidden="true"
    >{{ initials }}</span>
    <div class="account-summary__text">
      <strong
        class="account-summary__name"
        :title="name"
      >{{ name || t('account.signedIn') }}</strong>
      <span
        v-if="handle"
        class="account-summary__handle"
      >{{ handle }}</span>
      <span class="account-summary__status">
        <span
          class="account-summary__dot"
          aria-hidden="true"
        />
        {{ t('account.signedIn') }}
        <span
          v-if="planName"
          class="account-summary__plan"
        >{{ planName }}</span>
      </span>
    </div>
    <BaseIconButton
      variant="default"
      :label="t('account.signOut')"
      :disabled="account.working"
      @click="account.logout()"
    >
      <IconLogOut />
    </BaseIconButton>
  </section>
</template>

<style scoped>
.account-summary {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4);
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}
.account-summary__avatar {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--color-accent);
  color: var(--color-text-on-accent, #fff);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}
.account-summary__text { display: grid; flex: 1; min-width: 0; gap: 2px; }
.account-summary__name { overflow: hidden; font-size: var(--font-size-md); text-overflow: ellipsis; white-space: nowrap; }
.account-summary__handle { overflow: hidden; color: var(--color-text-muted); font-size: var(--font-size-sm); text-overflow: ellipsis; white-space: nowrap; }
.account-summary__status { display: flex; align-items: center; gap: var(--space-2); color: var(--color-text-muted); font-size: var(--font-size-sm); }
.account-summary__dot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-success); }
.account-summary__plan { padding: 0 var(--space-2); background: var(--color-bg-control); color: var(--color-text); font-size: var(--font-size-xs); font-weight: var(--font-weight-semibold); }
@container (width < 600px) {
  .account-summary :deep(.base-icon-button) { width: 44px; height: 44px; }
}
</style>
