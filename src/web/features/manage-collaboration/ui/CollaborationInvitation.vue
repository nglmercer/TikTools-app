<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref, watch } from 'vue';
import type { Grant, InvitationSecret } from '@/entities/collaboration/api/types.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import CollabSection from './CollabSection.vue';
import GrantEditor from './GrantEditor.vue';
import InvitationQr from './InvitationQr.vue';
const props = defineProps<{ blocked: boolean; canInvite: boolean; joinLink: string; secret: InvitationSecret | null }>();
const grant = defineModel<Grant>({ required: true });
const emit = defineEmits<{ create: []; hide: []; error: [message: string]; navigate: [area: 'direct'] }>();
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
  <CollabSection
    :title="t('collaboration.invites')"
    :description="t('collaboration.inviteHint')"
  >
    <GrantEditor
      v-model="grant"
      :disabled="blocked"
    />
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
        <BaseButton
          variant="primary"
          @click="copyLink"
        >
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
    <template #footer>
      <div class="collab-actionbar">
        <p
          v-if="!canInvite"
          class="collab-actionbar__note"
        >
          {{ t('collaboration.listenerRequired') }}
          <BaseButton
            size="sm"
            variant="ghost"
            @click="emit('navigate', 'direct')"
          >
            {{ t('collaboration.openDirect') }}
          </BaseButton>
        </p>
        <BaseButton
          class="collab-actionbar__primary"
          variant="primary"
          :disabled="!canInvite"
          @click="emit('create')"
        >
          {{ t('collaboration.create') }}
        </BaseButton>
      </div>
    </template>
  </CollabSection>
</template>
