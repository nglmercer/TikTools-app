<script setup lang="ts">
import { ref } from 'vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';

const base = {
  schemaVersion: 2,
  id: 'evt-gift-rose',
  name: 'Rose gift alert',
  enabled: true,
  trigger: 'tiktok.gift',
  conditions: [{ path: 'event.data.giftName', op: 'eq', value: 'Rose' }],
  actionIds: ['alert.action', 'thanks.action'],
  cooldown: { scope: 'user', ms: 5000 },
  note: null,
};
const local = {
  ...base,
  name: 'Rose gift alert (loud)',
  cooldown: { scope: 'user', ms: 2000 },
  actionIds: ['alert.action', 'thanks.action', 'confetti.action'],
};
const cloud = {
  ...base,
  enabled: false,
  conditions: [{ path: 'event.data.giftName', op: 'in', value: ['Rose', 'Heart'] }],
};
const large = { items: Array.from({ length: 400 }, (_, index) => ({ id: index, label: `Item ${index}`, active: index % 3 === 0 })) };
const layout = ref<'unified' | 'side-by-side'>('side-by-side');
</script>

<template>
  <section class="ui-kit-json">
    <header class="ui-kit-json__header">
      JSON viewing
    </header>
    <div class="ui-kit-json__body">
      <div>
        <h3>BaseJsonView · code (line numbers, wrap, copy)</h3>
        <BaseJsonView
          :value="base"
          line-numbers
          max-height="18rem"
          switchable
        />
      </div>
      <div>
        <h3>BaseJsonView · tree (collapsible, counts)</h3>
        <BaseJsonView
          :value="base"
          mode="tree"
          max-height="18rem"
          switchable
        />
      </div>
      <div>
        <h3>Invalid JSON text and large documents</h3>
        <div class="ui-kit-json__stack">
          <BaseJsonView
            source="{&quot;name&quot;: &quot;broken&quot;, oops"
            max-height="8rem"
          />
          <BaseJsonView
            :value="large"
            :max-lines="120"
            line-numbers
            max-height="10rem"
          />
        </div>
      </div>
    </div>
    <div class="ui-kit-json__body ui-kit-json__body--diffs">
      <div>
        <h3>BaseJsonDiff · base to local (unified)</h3>
        <BaseJsonDiff
          :before="base"
          :after="local"
          :context="2"
          max-height="22rem"
          switchable
        />
      </div>
      <div>
        <h3>BaseJsonDiff · base to cloud (side by side)</h3>
        <BaseJsonDiff
          v-model:layout="layout"
          :before="base"
          :after="cloud"
          before-label="Base"
          after-label="Cloud"
          :context="2"
          max-height="22rem"
          switchable
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.ui-kit-json {
  display: flex;
  flex-direction: column;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.ui-kit-json__header {
  padding: var(--space-3) var(--space-4);
  background: var(--color-bg-panel-header);
  font-weight: var(--font-weight-semibold);
}

.ui-kit-json__body {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(320px, 100%), 1fr));
  gap: var(--space-4);
  padding: var(--space-4);
}

.ui-kit-json__body--diffs {
  grid-template-columns: repeat(auto-fit, minmax(min(480px, 100%), 1fr));
  padding-top: 0;
}

.ui-kit-json__body > div {
  min-width: 0;
}

.ui-kit-json__stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

h3 {
  margin: 0 0 var(--space-3);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}
</style>
