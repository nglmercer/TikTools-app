<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CollaborationStatus, Configuration } from '@/entities/collaboration/api/types.ts';
import { activeGateway } from '@/entities/collaboration/lib/gateway.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseTextArea from '@/shared/ui/text-area/BaseTextArea.vue';
import CollabSection from './CollabSection.vue';

const props = defineProps<{ status: CollaborationStatus; blocked: boolean; changed: boolean; working: boolean; available: boolean; dirty: boolean }>();
const draft = defineModel<Configuration>({ required: true });
const allowedOrigins = defineModel<string>('allowedOrigins', { required: true });
const emit = defineEmits<{ apply: []; reset: []; teams: [] }>();
const { t } = useI18n();
const native = computed(() => !!props.status.peerGateway?.available);

function setText(key: 'bindAddress' | 'advertisedOrigin' | 'tlsCertificate' | 'tlsPrivateKey', value: string): void {
  if (key === 'bindAddress') draft.value.bindAddress = value;
  else draft.value[key] = value || null;
}
const setProxy = (proxy: boolean): void => { draft.value.tlsProxy = proxy; };

type Tone = 'success' | 'warning' | 'error';
// Only checks the browser can honestly derive from the draft and the host's reported state: nothing here reads the file system.
const checks = computed<Array<{ key: string; tone: Tone; text: string }>>(() => {
  const list: Array<{ key: string; tone: Tone; text: string }> = [];
  const gateway = activeGateway(props.status);
  if (gateway.lastError) list.push({ key: 'error', tone: 'error', text: gateway.lastError });
  else if (gateway.running) list.push({ key: 'listener', tone: 'success', text: t('collaboration.checkListening') });
  else list.push({ key: 'listener', tone: 'warning', text: t('collaboration.checkStopped') });
  if (native.value) return list;
  const origin = draft.value.advertisedOrigin ?? '';
  list.push(origin.startsWith('https://') ? { key: 'https', tone: 'success', text: t('collaboration.checkHttps') } : { key: 'https', tone: 'warning', text: t('collaboration.checkNoPublic') });
  if (draft.value.tlsProxy) list.push({ key: 'tls', tone: 'success', text: t('collaboration.checkProxy') });
  else if (draft.value.tlsCertificate && draft.value.tlsPrivateKey) list.push({ key: 'tls', tone: 'success', text: t('collaboration.checkTlsFiles') });
  else list.push({ key: 'tls', tone: 'warning', text: t('collaboration.checkTlsMissing') });
  const host = draft.value.bindAddress.trim().replace(/:\d+$/, '').replace(/^\[|\]$/g, '');
  if (host !== '' && !['127.0.0.1', 'localhost', '::1'].includes(host)) list.push({ key: 'wide', tone: 'warning', text: t('collaboration.warnAllInterfaces') });
  return list;
});
const markFor = (tone: Tone): string => (tone === 'success' ? '✓' : tone === 'warning' ? '!' : '×');
const preview = computed(() => `${draft.value.advertisedOrigin || 'https://…'}/remote/v1/join#…`);
</script>

<template>
  <form
    class="connection"
    autocomplete="off"
    @submit.prevent="emit('apply')"
  >
    <div class="connection__form">
      <label class="connection__toggle">
        <span class="connection__toggle-text">
          <strong>{{ t('collaboration.enabled') }}</strong>
          <span class="collaboration-editor__muted">{{ native ? t('cloudTeams.hint') : available ? t('collaboration.enabledHint') : t('collaboration.unavailableDetail') }}</span>
        </span>
        <BaseSwitch
          :model-value="draft.enabled"
          :disabled="blocked || !available || (native && !draft.peer)"
          :aria-label="t('collaboration.enabled')"
          @update:model-value="draft.enabled = $event"
        />
      </label>
      <BaseButton
        v-if="native"
        @click="emit('teams')"
      >
        {{ t('cloudTeams.title') }}
      </BaseButton>
      <CollabSection
        v-if="!native"
        :title="t('collaboration.whereConnect')"
      >
        <div class="connection__pair">
          <div class="connection__field">
            <label for="collaboration-bind">{{ t('collaboration.bind') }}</label>
            <BaseTextField
              id="collaboration-bind"
              :model-value="draft.bindAddress"
              maxlength="256"
              :disabled="blocked"
              @update:model-value="setText('bindAddress', $event)"
            />
            <span class="collaboration-editor__muted">{{ t('collaboration.bindHint') }}</span>
          </div>
          <div class="connection__field">
            <label for="collaboration-origin">{{ t('collaboration.origin') }}</label>
            <BaseTextField
              id="collaboration-origin"
              :model-value="draft.advertisedOrigin ?? ''"
              maxlength="2048"
              :disabled="blocked"
              placeholder="https://pc-a.example:8788"
              @update:model-value="setText('advertisedOrigin', $event)"
            />
            <span class="collaboration-editor__muted">{{ t('collaboration.originHint') }}</span>
          </div>
        </div>
      </CollabSection>
      <CollabSection
        v-if="!native"
        :title="t('collaboration.tlsTitle')"
      >
        <div
          class="connection__modes"
          role="radiogroup"
          :aria-label="t('collaboration.tlsTitle')"
        >
          <label
            class="connection__mode"
            :class="{ 'connection__mode--on': !draft.tlsProxy }"
          >
            <input
              type="radio"
              name="collaboration-tls"
              :checked="!draft.tlsProxy"
              :disabled="blocked"
              @change="setProxy(false)"
            >
            <span><strong>{{ t('collaboration.tlsOwn') }}</strong><span class="collaboration-editor__muted">{{ t('collaboration.tlsOwnHint') }}</span></span>
          </label>
          <label
            class="connection__mode"
            :class="{ 'connection__mode--on': draft.tlsProxy }"
          >
            <input
              type="radio"
              name="collaboration-tls"
              :checked="draft.tlsProxy"
              :disabled="blocked"
              @change="setProxy(true)"
            >
            <span><strong>{{ t('collaboration.tlsProxy') }}</strong><span class="collaboration-editor__muted">{{ t('collaboration.proxyHint') }}</span></span>
          </label>
        </div>
        <div
          v-if="!draft.tlsProxy"
          class="connection__pair"
        >
          <div class="connection__field">
            <label for="collaboration-certificate">{{ t('collaboration.certificate') }}</label>
            <BaseTextField
              id="collaboration-certificate"
              :model-value="draft.tlsCertificate ?? ''"
              :disabled="blocked"
              maxlength="4096"
              @update:model-value="setText('tlsCertificate', $event)"
            />
          </div>
          <div class="connection__field">
            <label for="collaboration-key">{{ t('collaboration.privateKey') }}</label>
            <BaseTextField
              id="collaboration-key"
              :model-value="draft.tlsPrivateKey ?? ''"
              :disabled="blocked"
              maxlength="4096"
              @update:model-value="setText('tlsPrivateKey', $event)"
            />
          </div>
          <span class="collaboration-editor__muted connection__wide">{{ t('collaboration.pathsHint') }}</span>
        </div>
      </CollabSection>
      <CollabSection
        v-if="!native"
        :title="t('collaboration.originsTitle')"
      >
        <label
          class="visually-hidden"
          for="collaboration-origins"
        >{{ t('collaboration.originsTitle') }}</label>
        <BaseTextArea
          id="collaboration-origins"
          v-model="allowedOrigins"
          :rows="3"
          maxlength="16384"
          :disabled="blocked"
          :placeholder="t('collaboration.origins')"
        />
      </CollabSection>
      <div
        class="collab-actionbar collab-actionbar--sticky"
        :class="{ 'collab-actionbar--dirty': dirty }"
      >
        <p
          class="collab-actionbar__note"
          :role="dirty ? 'status' : undefined"
        >
          <template v-if="dirty">
            {{ t('collaboration.unsaved') }}
          </template>
        </p>
        <BaseButton
          :disabled="working"
          @click="emit('reset')"
        >
          {{ t('collaboration.reset') }}
        </BaseButton>
        <BaseButton
          type="submit"
          variant="primary"
          :disabled="blocked || changed || !available"
        >
          {{ t('collaboration.save') }}
        </BaseButton>
      </div>
    </div>
    <aside
      class="connection__side"
      :aria-label="t('collaboration.checksTitle')"
    >
      <CollabSection :title="t('collaboration.checksTitle')">
        <ul class="connection__checks">
          <li
            v-for="check in checks"
            :key="check.key"
            class="connection__check"
          >
            <span
              class="connection__mark"
              :class="`connection__mark--${check.tone}`"
              aria-hidden="true"
            >{{ markFor(check.tone) }}</span>
            <span>{{ check.text }}</span>
          </li>
        </ul>
      </CollabSection>
      <CollabSection
        v-if="!native"
        :title="t('collaboration.linkPreview')"
      >
        <code class="connection__preview">{{ preview }}</code>
        <p
          v-if="status.gateway.running && status.gateway.certificateFingerprint"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.fingerprint') }}: <code class="collaboration-editor__fingerprint">{{ status.gateway.certificateFingerprint }}</code>
        </p>
        <p
          v-else-if="draft.tlsProxy"
          class="collaboration-editor__muted"
        >
          {{ t('collaboration.proxyFingerprint') }}
        </p>
      </CollabSection>
    </aside>
  </form>
</template>
