<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { friendlyAccountError } from '@/entities/cloud-account/lib/account-error.ts';
import CloudPlanUsage from '@/features/cloud-account/ui/CloudPlanUsage.vue';

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
const loginPending = computed(() => state.value === 'signing-in' || account.loginActive);
const showCodeLogin = ref(false);
const codeEmail = ref('');
const codeValue = ref('');

onMounted(() => {
  void account.ensureRestored();
});
</script>

<template>
  <BaseGroupBox
    :title="t('account.group')"
    :description="state === 'signed-in' ? undefined : t('account.description')"
  >
    <div
      class="cloud-account"
      :aria-busy="loginPending || undefined"
    >
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
        <SettingsRow
          :label="name ?? t('account.signedIn')"
          :description="handle ?? undefined"
        >
          <template #status>
            <BaseStatus
              :label="t('account.signedIn')"
              tone="success"
            />
          </template>
          <BaseButton
            :disabled="account.working"
            @click="account.logout()"
          >
            {{ t('account.signOut') }}
          </BaseButton>
        </SettingsRow>
        <CloudPlanUsage />
        <DiagnosticStatusAction
          v-if="error"
          :label="error"
          source="account"
        />
      </template>
      <template v-else>
        <DiagnosticStatusAction
          v-if="error"
          :label="error"
          source="account"
        />
        <template v-if="account.loginCode && !account.loginCode.requiresOauth">
          <p class="cloud-account__muted">
            {{ t('account.codeSent', { count: account.loginCode.delivered }) }}
          </p>
          <div class="cloud-account__actions">
            <input
              v-model="codeValue"
              class="cloud-account__code"
              inputmode="numeric"
              maxlength="6"
              :placeholder="t('account.codePlaceholder')"
              autocomplete="one-time-code"
              @keyup.enter="account.submitLoginCode(codeValue)"
            >
            <BaseButton
              variant="primary"
              :disabled="account.working || !/^\d{6}$/.test(codeValue)"
              @click="account.submitLoginCode(codeValue)"
            >
              {{ t('account.signIn') }}
            </BaseButton>
            <BaseButton
              :disabled="account.working"
              @click="account.resendLoginCode()"
            >
              {{ t('account.resend') }}
            </BaseButton>
          </div>
        </template>
        <template v-else-if="showCodeLogin && state !== 'signing-in' && !account.loginActive">
          <div class="cloud-account__actions">
            <input
              v-model="codeEmail"
              class="cloud-account__code"
              type="email"
              :placeholder="t('account.emailPlaceholder')"
              @keyup.enter="account.startLoginCode(codeEmail)"
            >
            <BaseButton
              variant="primary"
              :disabled="account.working || !codeEmail.includes('@')"
              @click="account.startLoginCode(codeEmail)"
            >
              {{ t('account.sendCode') }}
            </BaseButton>
          </div>
        </template>
        <template v-if="loginPending">
          <p
            class="cloud-account__pending"
            role="status"
            aria-live="polite"
          >
            <IconRefresh
              class="cloud-account__spin"
              :size="16"
              aria-hidden="true"
            />
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
          <template v-if="state !== 'signing-in' && !account.loginActive && !account.loginCode">
            <BaseButton
              variant="primary"
              :disabled="account.working"
              @click="account.startLogin()"
            >
              {{ t('account.signIn') }}
            </BaseButton>
            <BaseButton
              :disabled="account.working"
              @click="showCodeLogin = !showCodeLogin"
            >
              {{ showCodeLogin ? t('account.cancel') : t('account.signInWithCode') }}
            </BaseButton>
          </template>
          <BaseButton
            v-else
            :disabled="account.working && !loginPending"
            @click="account.cancelLogin(); showCodeLogin = false; account.loginCode = null"
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

.cloud-account__muted {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.cloud-account__pending {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.cloud-account__spin {
  flex: none;
  color: var(--color-accent);
  animation: cloud-account-spin 1s linear infinite;
}

@keyframes cloud-account-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  .cloud-account__spin { animation: none; }
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
  align-items: center;
}

.cloud-account__code {
  flex: 1;
  min-width: 0;
  padding: var(--space-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-text);
  font-family: var(--font-family-mono);
  letter-spacing: 0.1em;
}
</style>
