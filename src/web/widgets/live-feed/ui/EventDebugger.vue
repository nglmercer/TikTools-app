<script setup lang="ts">
import { ref, shallowRef } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
const props = defineProps<{ renderedCount: number }>();
const control = useControlClient();
const feed = useLiveFeedStore();
const snapshot = shallowRef<unknown>(null);
const pending = ref(false);
async function refresh(): Promise<void> {
  if (pending.value) return;
  pending.value = true;
  try {
    const health = await control.call<Record<string, unknown>>('system.health', {});
    snapshot.value = {
      ingress: health.liveIngress, dedup: health.liveDedup, totals: health.liveTotals,
      pointsQueue: health.livePointsWork, delivery: health.desktop,
      frontend: { ...feed.diagnostics(), rendered: props.renderedCount },
      // Public identity metadata only; never dump event payloads or credentials.
      recent: feed.events.slice(-12).map((e) => ({ correlationId: e.correlationId, cursor: e.feedCursor, msgId: e.msgId, type: e.sourceType ?? e.kind, room: e.roomId, creator: e.creatorId })),
    };
  } catch { snapshot.value = { error: 'Diagnostics unavailable' }; }
  finally { pending.value = false; }
}
</script>
<template>
  <details
    class="event-debugger"
    @toggle="($event.target as HTMLDetailsElement).open && refresh()"
  >
    <summary>Event Debugger</summary>
    <button
      type="button"
      :disabled="pending"
      @click="refresh"
    >
      Refresh
    </button>
    <pre aria-live="polite">{{ JSON.stringify(snapshot, null, 2) }}</pre>
  </details>
</template>
<style scoped>
.event-debugger { padding: 8px 12px; font-size: 12px; }
pre { max-height: 280px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
