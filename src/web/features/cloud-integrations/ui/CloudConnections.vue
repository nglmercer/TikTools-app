<script setup lang="ts">
import { failureText } from '@/shared/api/cloud-failure.ts';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { isValidLabel, useIntegrationsStore } from '@/entities/cloud-integrations/model/integrations-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const { t } = useI18n();
const integrations = useIntegrationsStore();
const account = useAccountStore();

const label = ref('');
const copied = ref(false);
const confirmId = ref<string | null>(null);

const labelValid = computed(() => isValidLabel(label.value));
const canCreate = computed(() => labelValid.value && !integrations.working);

/** The full delivery URL, built from the Cloud origin the account is signed in to. */
const deliveryUrl = computed(() => {
  const issued = integrations.issued;
  if (!issued) return '';
  const origin = account.status?.origin ?? '';
  return origin ? `${origin.replace(/\/$/, '')}${issued.deliveryPath}` : issued.deliveryPath;
});

function formatPayload(payload: unknown): string {
  const text = JSON.stringify(payload, null, 2) ?? '';
  return text.length > 4000 ? `${text.slice(0, 4000)}…` : text;
}

async function create(): Promise<void> {
  if (!canCreate.value) return;
  const created = await integrations.connect(label.value);
  if (created) {
    label.value = '';
    copied.value = false;
  }
}

async function copySecret(): Promise<void> {
  const secret = integrations.issued?.secret;
  if (!secret) return;
  try {
    await navigator.clipboard.writeText(secret);
    copied.value = true;
  } catch {
    copied.value = false;
  }
}

function askRevoke(id: string): void {
  confirmId.value = id;
}

async function revoke(id: string): Promise<void> {
  confirmId.value = null;
  await integrations.remove(id);
}

onMounted(() => {
  void integrations.refresh();
});
onBeforeUnmount(() => {
  // The secret must not outlive the panel.
  integrations.reset();
});
</script>

<template>
  <section class="cloud-connections">
    <h3 class="cloud-connections__title">
      {{ t('cloudConnections.title') }}
    </h3>
    <p class="cloud-connections__hint">
      {{ t('cloudConnections.hint') }}
    </p>

    <form
      class="cloud-connections__create"
      @submit.prevent="create"
    >
      <BaseTextField
        v-model="label"
        class="cloud-connections__label"
        :placeholder="t('cloudConnections.labelPlaceholder')"
        :aria-label="t('cloudConnections.label')"
        :aria-invalid="label.length > 0 && !labelValid"
        maxlength="200"
      />
      <BaseButton
        type="submit"
        variant="primary"
        :disabled="!canCreate"
      >
        {{ t('cloudConnections.create') }}
      </BaseButton>
    </form>
    <p
      v-if="label.length > 0 && !labelValid"
      class="cloud-connections__note"
    >
      {{ t('cloudConnections.labelRule') }}
    </p>

    <BaseAlert
      v-if="integrations.issued"
      tone="warning"
      :title="t('cloudConnections.secretOnce')"
      class="cloud-connections__secret"
    >
      <p>{{ t('cloudConnections.deliveryUrl') }}</p>
      <code class="cloud-connections__code">{{ deliveryUrl }}</code>
      <p>{{ t('cloudConnections.secret') }}</p>
      <code class="cloud-connections__code">{{ integrations.issued.secret }}</code>
      <div class="cloud-connections__actions">
        <BaseButton
          size="sm"
          @click="copySecret"
        >
          {{ copied ? t('cloudConnections.copied') : t('cloudConnections.copy') }}
        </BaseButton>
        <BaseButton
          size="sm"
          variant="ghost"
          @click="integrations.dismissSecret()"
        >
          {{ t('cloudConnections.dismiss') }}
        </BaseButton>
      </div>
    </BaseAlert>

    <BaseAlert
      v-if="integrations.error"
      tone="error"
      :title="failureText(t, integrations.errorDetail, integrations.error)"
      compact
    />

    <BaseEmptyState
      v-if="!integrations.loading && integrations.connections.length === 0"
      :title="t('cloudConnections.empty')"
    />

    <ul
      v-else
      class="cloud-connections__list"
    >
      <li
        v-for="connection in integrations.connections"
        :key="connection.id"
        class="cloud-connections__row"
      >
        <div class="cloud-connections__meta">
          <strong>{{ connection.label }}</strong>
          <span>{{ t(`cloudConnections.status.${connection.status}`) }}</span>
          <span>{{ connection.createdAt }}</span>
        </div>
        <div class="cloud-connections__actions">
          <BaseButton
            size="sm"
            :pressed="integrations.selectedId === connection.id"
            @click="integrations.openEvents(connection.id)"
          >
            {{ t('cloudConnections.events') }}
          </BaseButton>
          <template v-if="connection.status === 'active'">
            <BaseButton
              v-if="confirmId !== connection.id"
              size="sm"
              variant="danger"
              @click="askRevoke(connection.id)"
            >
              {{ t('cloudConnections.disconnect') }}
            </BaseButton>
            <template v-else>
              <BaseButton
                size="sm"
                variant="danger"
                @click="revoke(connection.id)"
              >
                {{ t('cloudConnections.confirmDisconnect') }}
              </BaseButton>
              <BaseButton
                size="sm"
                variant="ghost"
                @click="confirmId = null"
              >
                {{ t('cloudConnections.cancel') }}
              </BaseButton>
            </template>
          </template>
        </div>
      </li>
    </ul>

    <section
      v-if="integrations.selectedId"
      class="cloud-connections__events"
      :aria-label="t('cloudConnections.events')"
    >
      <header class="cloud-connections__actions">
        <strong>{{ t('cloudConnections.recentEvents') }}</strong>
        <BaseButton
          size="sm"
          variant="ghost"
          @click="integrations.closeEvents()"
        >
          {{ t('cloudConnections.close') }}
        </BaseButton>
      </header>
      <BaseAlert
        v-if="integrations.eventsError"
        tone="error"
        :title="integrations.eventsError"
        compact
      />
      <p
        v-else-if="integrations.events && integrations.events.items.length === 0"
      >
        {{ t('cloudConnections.noEvents') }}
      </p>
      <ol
        v-else-if="integrations.events"
        class="cloud-connections__list"
      >
        <li
          v-for="event in integrations.events.items"
          :key="event.deliveryId"
        >
          <span>{{ event.receivedAt }}</span>
          <pre>{{ formatPayload(event.payload) }}</pre>
        </li>
      </ol>
      <p v-if="integrations.events?.truncated">
        {{ t('cloudConnections.truncated') }}
      </p>
    </section>
  </section>
</template>

<style scoped>
.cloud-connections { display:grid; gap:var(--space-3); min-width:0; }
.cloud-connections__title { margin:0; }
.cloud-connections__hint, .cloud-connections__note { margin:0; color:var(--color-text-muted); }
.cloud-connections__create { display:flex; flex-wrap:wrap; gap:var(--space-2); }
.cloud-connections__label { flex:1 1 220px; min-width:0; }
.cloud-connections__code { display:block; overflow-wrap:anywhere; padding:var(--space-2); border-radius:var(--radius-sm); background:var(--color-surface-raised); }
.cloud-connections__secret p { margin:var(--space-1) 0; }
.cloud-connections__list { list-style:none; margin:0; padding:0; display:grid; gap:var(--space-2); }
.cloud-connections__row { display:flex; flex-wrap:wrap; justify-content:space-between; gap:var(--space-2); align-items:center; min-width:0; }
.cloud-connections__meta { display:grid; gap:2px; min-width:0; overflow-wrap:anywhere; }
.cloud-connections__meta span { font-size:var(--font-size-sm); color:var(--color-text-muted); }
.cloud-connections__actions { display:flex; flex-wrap:wrap; gap:var(--space-2); align-items:center; }
.cloud-connections__events { display:grid; gap:var(--space-2); }
.cloud-connections__events pre { white-space:pre-wrap; overflow-wrap:anywhere; max-height:200px; overflow:auto; margin:0; padding:var(--space-2); border-radius:var(--radius-sm); background:var(--color-surface-raised); }
</style>
