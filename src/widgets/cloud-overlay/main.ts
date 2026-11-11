import { createApp, h, reactive } from 'vue';
import CloudOverlay from './CloudOverlay.vue';
import { decodeOverlayFragment } from '../../shared/cloud-overlay-fragment.ts';
import { validateOverlay, type OverlayDocument } from '../../web/entities/overlay/lib/cloud-overlay-schema.ts';
import { createWidgetRuntime } from '../shared/widget-runtime.ts';
import { liveFieldsFrom, type LiveFieldValues } from '../shared/cloud-overlay-fields.ts';
import { DEFAULT_TOPICS } from '../shared/config.ts';
import './cloud-overlay.css';

/**
 * Renders one cloud overlay document from the OBS URL fragment. The document is
 * checked with the same rules the Cloud applies; an invalid one shows nothing rather
 * than a partial layout. Live fields come from the gateway through the shared runtime.
 */
const decoded = decodeOverlayFragment(window.location.hash);
const document = decoded as OverlayDocument | null;
const valid = document !== null && typeof document === 'object' && validateOverlay(document).length === 0;
const values = reactive<LiveFieldValues>({});

createApp({
  render: () => h(CloudOverlay, { document: valid ? document : null, values }),
}).mount('#app');

if (valid) {
  const runtime = createWidgetRuntime({
    topics: DEFAULT_TOPICS,
    onEnvelope: (envelope) => {
      Object.assign(values, liveFieldsFrom(envelope));
    },
  });
  runtime.start();
  window.addEventListener('pagehide', () => runtime.stop(), { once: true });
}
