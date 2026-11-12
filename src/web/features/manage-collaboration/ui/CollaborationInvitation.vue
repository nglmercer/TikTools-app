<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref, watch } from 'vue';
import type { Grant, InvitationSecret } from '@/entities/collaboration/api/types.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import GrantEditor from './GrantEditor.vue';
import InvitationQr from './InvitationQr.vue';
const props = defineProps<{ blocked: boolean; canInvite: boolean; joinLink: string; secret: InvitationSecret | null }>();
const grant = defineModel<Grant>({ required: true });
const emit = defineEmits<{ create: []; hide: []; error: [message: string] }>();
const { t } = useI18n();
const { date } = useCollaborationPresentation();
const showQr = ref(false);
const copied = ref(false);
watch(() => props.secret, () => { showQr.value = false; copied.value = false; });
async function copyLink(): Promise<void> {
  try { await navigator.clipboard.writeText(props.joinLink); copied.value = true; }
  catch { emit('error', t('collaboration.clipboard')); }
}
</script>

<template>
  <BaseGroupBox
    :title="t('collaboration.invites')"
    :description="t('collaboration.inviteHint')"
  >
    <GrantEditor
      v-model="grant"
      :disabled="blocked"
    />
    <p
      v-if="!canInvite"
      class="collaboration-editor__muted"
    >
      {{ t('collaboration.listenerRequired') }}
    </p>
    <div class="collaboration-editor__actions">
      <BaseButton
        variant="primary"
        :disabled="!canInvite"
        @click="emit('create')"
      >
        {{ t('collaboration.create') }}
      </BaseButton>
    </div>
    <section
      v-if="secret && joinLink"
      class="collaboration-editor__share"
    >
      <p class="collaboration-editor__muted">
        {{ t('collaboration.shareHint') }}
      </p>
      <label for="collaboration-link">{{ t('collaboration.privateLink') }}</label>
      <BaseTextField
        id="collaboration-link"
        :model-value="joinLink"
        type="password"
        readonly
        revealable
        autocomplete="off"
      />
      <p class="collaboration-editor__muted">
        {{ t('collaboration.expires', { when: date(secret.expiresAt) }) }}
      </p>
      <div class="collaboration-editor__actions">
        <BaseButton @click="copyLink">
          {{ copied ? t('collaboration.copied') : t('collaboration.copy') }}
        </BaseButton>
        <BaseButton @click="showQr = !showQr">
          {{ showQr ? t('collaboration.hideQr') : t('collaboration.qr') }}
        </BaseButton>
        <BaseButton @click="emit('hide')">
          {{ t('collaboration.hide') }}
        </BaseButton>
      </div>
      <InvitationQr
        v-if="showQr"
        :value="joinLink"
      />
    </section>
  </BaseGroupBox>
</template>
