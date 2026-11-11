<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import { sessionRecoveryKey } from '@/shared/api/session-recovery.ts';

const recovery = inject(sessionRecoveryKey, null);
const { t } = useI18n();
const dismissed = ref(false);

const expired = computed(() => recovery?.state.value === 'expired');
const state = computed(() => recovery?.state.value ?? 'ok');
watch(state, () => { dismissed.value = false; });

const open = computed({
  get: () => !!recovery && !dismissed.value && (expired.value || state.value === 'unavailable'),
  set: (value: boolean) => { if (!value) dismissed.value = true; },
});
const busy = computed(() => recovery?.busy.value === true);

function retry(): void { void recovery?.retryNow(); }
function pair(): void {
  if (!recovery) return;
  // The adapter serves the pair page for any unauthenticated navigation, and
  // the remote host serves its join page. Both start a fresh pairing.
  location.assign(recovery.pairUrl);
}
</script>

<template>
  <BaseDialog
    v-if="recovery"
    v-model:open="open"
    :title="expired ? t('sessionRecovery.expiredTitle') : t('sessionRecovery.unavailableTitle')"
    :width="440"
  >
    <p class="session-recovery__message">
      {{ expired ? t('sessionRecovery.expiredMessage') : t('sessionRecovery.unavailableMessage') }}
    </p>
    <p
      v-if="busy"
      class="session-recovery__hint"
      role="status"
    >
      {{ t('sessionRecovery.reconnecting') }}
    </p>
    <template #footer>
      <BaseButton
        v-if="expired"
        variant="primary"
        @click="pair"
      >
        {{ t('sessionRecovery.pair') }}
      </BaseButton>
      <BaseButton
        v-else
        variant="primary"
        :disabled="busy"
        @click="retry"
      >
        {{ busy ? t('sessionRecovery.reconnecting') : t('sessionRecovery.retry') }}
      </BaseButton>
      <BaseButton
        v-if="expired"
        :disabled="busy"
        @click="retry"
      >
        {{ busy ? t('sessionRecovery.reconnecting') : t('sessionRecovery.retry') }}
      </BaseButton>
      <BaseButton
        v-else
        @click="pair"
      >
        {{ t('sessionRecovery.pair') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.session-recovery__message {
  margin: 0;
  line-height: var(--line-height-normal);
}

.session-recovery__hint {
  margin: var(--space-3) 0 0;
  color: var(--color-text-muted);
}
</style>
