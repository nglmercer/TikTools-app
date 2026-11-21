<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { friendlyAccountError } from '@/entities/cloud-account/lib/account-error.ts';
import CloudAccountSummary from '@/features/cloud-account/ui/CloudAccountSummary.vue';
import CloudPlanUsage from '@/features/cloud-account/ui/CloudPlanUsage.vue';
import CloudBackupSection from '@/features/cloud-account/ui/CloudBackupSection.vue';

const { t } = useI18n();
const account = useAccountStore();

const state = computed(() => account.status?.state ?? null);
const error = computed(() => friendlyAccountError(account.error, t));
const loginPending = computed(() => state.value === 'signing-in' || account.loginActive);
const linkCopied = ref(false);
async function copyLoginUrl(): Promise<void> {
  if (!account.loginUrl) return;
  try {
    await navigator.clipboard.writeText(account.loginUrl);
    linkCopied.value = true;
  } catch { /* clipboard can be blocked; the link stays selectable */ }
}
const showCodeLogin = ref(false);
const codeEmail = ref('');
const codeValue = ref('');

onMounted(() => {
  void account.ensureRestored();
});
</script>

<template>
  <div
    v-if="state === 'signed-in'"
    class="cloud-workspace"
    data-guide-scope="account"
  >
    <div class="cloud-workspace__grid">
      <aside class="cloud-workspace__aside">
        <CloudAccountSummary />
        <CloudPlanUsage />
      </aside>
      <div class="cloud-workspace__main">
        <CloudBackupSection />
      </div>
    </div>
    <DiagnosticStatusAction
      v-if="error"
      :label="error"
      source="account"
    />
  </div>
  <BaseGroupBox
    v-else
    :title="t('account.group')"
    :description="t('account.description')"
  >
    <div
      class="cloud-account"
      data-guide-scope="account"
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
        <section
          v-if="loginPending"
          class="cloud-account__waiting"
          role="status"
          aria-live="polite"
        >
          <div class="cloud-account__waiting-head">
            <span class="cloud-account__waiting-spinner">
              <IconRefresh
                class="cloud-account__spin"
                :size="16"
                aria-hidden="true"
              />
            </span>
            <div>
              <strong>{{ t('account.waiting') }}</strong>
              <p>{{ t('account.openManually') }}</p>
            </div>
          </div>
          <div
            v-if="account.loginUrl"
            class="cloud-account__waiting-url"
          >
            <code>{{ account.loginUrl }}</code>
            <BaseButton
              size="sm"
              @click="copyLoginUrl"
            >
              <template #icon>
                <IconCopy :size="14" />
              </template>
              {{ linkCopied ? t('account.linkCopied') : t('account.copyLink') }}
            </BaseButton>
          </div>
        </section>
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
/* Two columns once there is room (summary left, backups right); one column below that. */
.cloud-workspace { container-type: inline-size; display: flex; flex-direction: column; gap: var(--space-3); min-width: 0; }
.cloud-workspace__grid { display: grid; gap: var(--space-4); align-items: stretch; }
.cloud-workspace__aside { display: grid; gap: var(--space-4); min-width: 0; align-content: start; }
.cloud-workspace__main { display: flex; flex-direction: column; min-width: 0; }
@container (760px <= width) {
  .cloud-workspace__grid { grid-template-columns: 300px minmax(0, 1fr); }
}

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

.cloud-account__waiting {
  display: grid;
  gap: var(--space-3);
  padding: var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-raised);
}

.cloud-account__waiting-head {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
}

.cloud-account__waiting-head strong { font-size: var(--font-size-md); }
.cloud-account__waiting-head p { margin: var(--space-1) 0 0; color: var(--color-text-muted); font-size: var(--font-size-sm); }

.cloud-account__waiting-spinner {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  color: var(--color-accent);
}

.cloud-account__waiting-url {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-field);
}

.cloud-account__waiting-url code {
  flex: 1;
  min-width: 0;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  overflow-wrap: anywhere;
  user-select: all;
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
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family-mono);
  letter-spacing: 0.1em;
}
</style>
