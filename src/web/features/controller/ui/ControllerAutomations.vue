<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { AutomationModel, AutomationRow, EventOperation } from '../model/automations.ts';
const props = defineProps<{ model: AutomationModel; epoch: string; busy: boolean }>();
const { tm } = useI18n();
const text = computed(() => tm('controller.automations.text') as MessageSchema['controller']['automations']['text']);
async function request(item: AutomationRow, operation: EventOperation): Promise<void> {
  const target = props.model.prepare(item, operation);
  if (target && props.model.current(target)) await props.model.change(target);
}
</script>

<template>
  <section
    v-if="model.allowed.value"
    class="automation-board"
    :aria-label="text.title"
  >
    <h2>{{ text.title }}</h2>
    <p>{{ text.note }}</p>
    <p
      v-if="model.error.value"
      role="alert"
    >
      {{ model.error.value }}
    </p>
    <p
      v-else-if="!model.ready.value"
      role="status"
    >
      {{ text.loading }}
    </p>
    <template v-if="model.ready.value">
      <p v-if="!model.rows.value.length">
        {{ text.empty }}
      </p>
      <ul>
        <li
          v-for="item in model.rows.value"
          :key="`${item.summary.kind}:${item.summary.id}`"
        >
          <div>
            <strong>{{ item.summary.name || item.summary.id }}</strong>
            <p>{{ item.summary.kind === 'event' ? text.event : text.action }} · {{ item.summary.id }}</p>
            <p>{{ item.summary.enabled ? text.enabled : text.disabled }} · {{ text.revision }} {{ item.version.revision }}</p>
          </div>
          <div class="automation-board__actions">
            <button
              v-if="item.summary.kind === 'action' && item.summary.enabled"
              :disabled="!model.canChange(item, 'disable-action')"
              @click="request(item, 'disable-action')"
            >
              {{ text.disableAction }}
            </button>
            <button
              v-if="!item.summary.enabled"
              :disabled="!model.canChange(item, item.summary.kind === 'event' ? 'enable-event' : 'enable-action')"
              @click="request(item, item.summary.kind === 'event' ? 'enable-event' : 'enable-action')"
            >
              {{ text.activate }}
            </button>
            <button
              v-if="item.summary.kind === 'event'"
              :disabled="!model.canChange(item, 'disable-event')"
              @click="request(item, 'disable-event')"
            >
              {{ text.disable }}
            </button>
            <button
              v-if="item.summary.kind === 'event'"
              :disabled="!model.canChange(item, 'delete-event')"
              @click="request(item, 'delete-event')"
            >
              {{ text.delete }}
            </button>
          </div>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.automation-board { padding:20px; margin-top:18px; border:1px solid var(--color-border-subtle); border-radius:8px; background:var(--color-bg-panel); overflow-wrap:anywhere; }
h2 { font-size:18px; } p { margin:8px 0; } ul { list-style:none; padding:0; margin:0; }
li { display:flex; gap:16px; align-items:center; justify-content:space-between; padding:14px 0; border-top:1px solid var(--color-border-subtle); }
li > div { min-width:0; }.automation-board__actions { display:flex; flex-wrap:wrap; gap:8px; flex-shrink:0; }
button { cursor:pointer; padding:8px 14px; border:1px solid var(--color-border-subtle); border-radius:6px; color:inherit; background:var(--color-bg-panel); }
button:disabled { opacity:.5; cursor:default; } code { display:block; overflow-wrap:anywhere; }
@media(max-width:700px) { li { align-items:stretch; flex-direction:column; }.automation-board__actions { flex-shrink:1; } }
</style>
