<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { friendlyAccountError } from '@/entities/cloud-account/lib/account-error.ts';

const { t } = useI18n();
const account = useAccountStore();

const state = computed(() => account.status?.state ?? null);
const name = computed(
  () =>
    account.status?.displayName
    ?? account.status?.email
    ?? account.status?.accountId
    ?? null,
);
/** Secondary line: the email, unless it is already the primary name. */
const handle = computed(() => {
  const email = account.status?.email ?? null;
  return email && email !== name.value ? email : null;
});
const error = computed(() => friendlyAccountError(account.error, t));

onMounted(() => {
  void account.ensureRestored();
});
</script>

<template>
  <BaseGroupBox
    :title="t('account.group')"
    :description="state === 'signed-in' ? undefined : t('account.description')"
  >
    <div class="cloud-account">
      <p
        v-if="account.loading && !account.status"
        class="cloud-account__muted"
      >
        {{ t('account.loading') }}
      </p>
      <p
        v-else-if="state === 'unconfigured'"
        class="cloud-account__muted"
      >
        {{ t('account.unconfigured') }}
      </p>
      <template v-else-if="state === 'signed-in'">
        <div class="cloud-account__identity">
          <span class="cloud-account__name">{{ name }}</span>
          <span
            v-if="handle"
            class="cloud-account__muted"
          >{{ handle }}</span>
          <span class="cloud-account__status">{{ t('account.signedIn') }}</span>
        </div>
        <BaseAlert
          v-if="error"
          :title="error"
          compact
        />
        <div class="cloud-account__actions">
          <BaseButton
            :disabled="account.working"
            @click="account.logout()"
          >
            {{ t('account.signOut') }}
          </BaseButton>
        </div>
      </template>
      <template v-else>
        <BaseAlert
          v-if="error"
          :title="error"
          compact
        />
        <template v-if="state === 'signing-in'">
          <p class="cloud-account__muted">
            {{ t('account.waiting') }}
          </p>
          <p
            v-if="account.loginUrl"
            class="cloud-account__muted"
          >
            {{ t('account.openManually') }}
            <span class="cloud-account__url">{{ account.loginUrl }}</span>
          </p>
        </template>
        <div class="cloud-account__actions">
          <BaseButton
            v-if="state !== 'signing-in'"
            variant="primary"
            :disabled="account.working"
            @click="account.startLogin()"
          >
            {{ t('account.signIn') }}
          </BaseButton>
          <BaseButton
            v-else
            @click="account.cancelLogin()"
          >
            {{ t('account.cancel') }}
          </BaseButton>
        </div>
      </template>
    </div>
  </BaseGroupBox>
</template>

<style scoped>
.cloud-account {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.cloud-account__identity {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.cloud-account__name {
  font-weight: var(--font-weight-semibold);
}

.cloud-account__status {
  color: var(--color-success);
  font-size: var(--font-size-sm);
}

.cloud-account__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.cloud-account__url {
  display: block;
  overflow-wrap: anywhere;
  font-family: var(--font-family-mono);
  user-select: all;
}

.cloud-account__actions {
  display: flex;
  gap: var(--space-3);
}
</style>
