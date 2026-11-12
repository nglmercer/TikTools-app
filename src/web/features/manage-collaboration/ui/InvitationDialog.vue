<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Grant, InvitationSecret } from '@/entities/collaboration/api/types.ts';
import { accessAreas, levelOf } from '../model/access-areas.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import InvitationQr from './InvitationQr.vue';

const props = defineProps<{ secret: InvitationSecret | null; joinLink: string; grant: Grant; fingerprint?: string | null; blocked: boolean }>();
const emit = defineEmits<{ close: []; revoke: [inviteId: string]; error: [message: string] }>();
const { t } = useI18n();
const { date } = useCollaborationPresentation();
const showQr = ref(false);
const copied = ref(false);
watch(() => props.secret, () => { showQr.value = false; copied.value = false; });
const open = computed({ get: () => !!props.secret && !!props.joinLink, set: (value: boolean) => { if (!value) emit('close'); } });
const chips = computed(() => accessAreas.map(area => ({ area, level: levelOf(props.grant, area) })).filter(item => item.level !== 'none'));
async function copyLink(): Promise<void> {
  try { await navigator.clipboard.writeText(props.joinLink); copied.value = true; }
  catch { emit('error', t('collaboration.clipboard')); }
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t('collaboration.inviteReady')"
    :width="720"
  >
    <div
      v-if="secret"
      class="invite-dialog"
    >
      <ul class="invite-dialog__chips">
        <li
          v-for="item in chips"
          :key="item.area.id"
        >
          {{ t(`collaboration.areas.${item.area.id}`) }}: {{ t(`collaboration.levels.${item.level}`) }}
        </li>
      </ul>
      <div class="invite-dialog__body">
        <div class="invite-dialog__qr">
          <InvitationQr
            v-if="showQr"
            :value="joinLink"
          />
          <BaseButton @click="showQr = !showQr">
            {{ showQr ? t('collaboration.hideQr') : t('collaboration.qr') }}
          </BaseButton>
        </div>
        <div class="invite-dialog__main">
          <label for="collaboration-link">{{ t('collaboration.privateLink') }}</label>
          <div class="invite-dialog__link">
            <BaseTextField
              id="collaboration-link"
              :model-value="joinLink"
              type="password"
              readonly
              revealable
              autocomplete="off"
            />
            <BaseButton
              variant="primary"
              @click="copyLink"
            >
              {{ copied ? t('collaboration.copied') : t('collaboration.copy') }}
            </BaseButton>
          </div>
          <span class="collaboration-editor__muted">{{ t('collaboration.expires', { when: date(secret.expiresAt) }) }}</span>
          <ol class="invite-dialog__steps">
            <li>{{ t('collaboration.shareStep1') }}</li>
            <li>{{ t('collaboration.shareStep2') }}</li>
            <li>{{ t('collaboration.shareStep3') }}</li>
          </ol>
          <p
            v-if="fingerprint"
            class="collaboration-editor__muted"
          >
            {{ t('collaboration.fingerprint') }}: <code class="collaboration-editor__fingerprint">{{ fingerprint }}</code>
          </p>
        </div>
      </div>
    </div>
    <template #footer-start>
      <BaseButton
        variant="danger"
        :disabled="blocked || !secret"
        @click="secret && emit('revoke', secret.inviteId)"
      >
        {{ t('collaboration.revokeInvite') }}
      </BaseButton>
    </template>
    <template #footer>
      <BaseButton
        variant="primary"
        @click="emit('close')"
      >
        {{ t('collaboration.done') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>
