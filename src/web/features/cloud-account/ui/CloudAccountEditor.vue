<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';

const { t } = useI18n();
const account = useAccountStore();

const state = computed(() => account.status?.state ?? null);
const origin = computed(() => account.status?.origin ?? null);
const accountId = computed(() => account.status?.accountId ?? null);
const persistence = computed(() => account.status?.persistence ?? null);

onMounted(() => {
  void account.refresh();
});
</script>

<template>
  <BaseGroupBox
    :title="t('account.group')"
    :description="t('account.description')"
  >
    <BaseAlert
      v-if="account.loading"
      tone="info"
      :title="t('account.loading')"
    />
    <template v-else>
      <BaseAlert
        v-if="state === 'unconfigured'"
        tone="info"
        :title="t('account.unconfigured')"
      />
      <template v-else-if="state === 'signed-in'">
        <p>
          {{ t('account.signedInAs', { account: accountId }) }}
        </p>
        <p>
          {{ t('account.origin', { origin }) }}
        </p>
        <p>
          {{ t(persistence === 'keychain' ? 'account.storedInKeychain' : 'account.storedInMemory') }}
        </p>
        <BaseAlert
          v-if="account.error"
          :title="account.error"
        />
        <BaseButton
          :disabled="account.working"
          @click="account.logout()"
        >
          {{ t('account.signOut') }}
        </BaseButton>
      </template>
      <template v-else>
        <BaseAlert
          v-if="account.error"
          :title="account.error"
        />
        <p v-if="state === 'signing-in'">
          {{ t('account.waiting') }}
        </p>
        <p v-if="account.loginUrl">
          {{ account.loginUrl }}
        </p>
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
          :disabled="account.working"
          @click="account.cancelLogin()"
        >
          {{ t('account.cancel') }}
        </BaseButton>
      </template>
    </template>
  </BaseGroupBox>
</template>
