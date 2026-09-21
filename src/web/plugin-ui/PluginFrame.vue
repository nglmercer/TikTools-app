<script lang="tsx">
import { onUnmounted, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { Locale } from '../i18n.ts';
import {
  PluginWebviewHost,
  type BrokerBackend,
  type BrokerEvent,
  type BrokerResponse,
} from './plugin-webview-host.ts';

type PluginFrameProps = {
  locale: Locale;
  pluginId: string;
  /** Plugin UI document URL (built `ui/dist/index.html`, served by the harness). */
  src: string;
  backend: BrokerBackend;
  /** Host event subscription source (plugin.* topics). */
  subscribeTopic?: (topic: string, listener: (data: unknown) => void) => () => void;
  onError?: (message: string) => void;
};

/**
 * Sandboxed frame for one plugin UI document (web host path).
 *
 * `sandbox="allow-scripts"` keeps the frame at an opaque origin: the
 * plugin document runs scripts but shares neither storage nor DOM with
 * the host, cannot navigate the top frame, and cannot open popups. All
 * host contact travels through the versioned broker envelope answered by
 * `PluginWebviewHost`, bound to this frame's `contentWindow`.
 */
export const PluginFrame = defineVueComponent<PluginFrameProps>(
  ['locale', 'pluginId', 'src', 'backend', 'subscribeTopic', 'onError'],
  (props) => {
    const frameRef = ref<HTMLIFrameElement | null>(null);
    let host: PluginWebviewHost | null = null;

    const onMessage = (event: MessageEvent): void => {
      const frame = frameRef.value?.contentWindow ?? null;
      if (!host || event.source !== frame) return;
      void host.handleFrameMessage(event.data, event.source);
    };

    const mountHost = (): void => {
      disposeHost();
      const frame = frameRef.value;
      if (!frame) return;
      const postToFrame = (envelope: BrokerResponse | BrokerEvent): void => {
        // Opaque-origin frame: no origin to allow-list, so post wide and
        // authenticate inbound traffic by `contentWindow` instead.
        frame.contentWindow?.postMessage(envelope, '*');
      };
      host = new PluginWebviewHost(frame.contentWindow, {
        pluginId: props.pluginId,
        backend: props.backend,
        postToFrame,
        subscribeTopic: props.subscribeTopic,
        onError: props.onError,
      });
    };

    const disposeHost = (): void => {
      host?.dispose();
      host = null;
    };

    watch(frameRef, (frame) => {
      if (frame) mountHost();
      else disposeHost();
    });

    // The broker binds one plugin id per frame: when the tab switches to a
    // different plugin page, Vue may reuse this component (and its iframe
    // element) with new props. Recreate the host so the new document talks
    // to a backend scoped to the new plugin — never to the previous page's
    // subscriptions and ownership. `mountHost` disposes the old host first.
    watch([() => props.src, () => props.pluginId], () => {
      if (frameRef.value) mountHost();
    });

    const attach = (): void => {
      window.addEventListener('message', onMessage);
      mountHost();
    };
    const detach = (): void => {
      window.removeEventListener('message', onMessage);
      disposeHost();
    };

    // Mount eagerly: the iframe exists on first render, and the broker
    // must be listening before the frame document boots.
    attach();
    onUnmounted(detach);

    return () => (
      <iframe
        ref={frameRef}
        class="plg-frame"
        title={props.pluginId}
        src={props.src}
        sandbox="allow-scripts"
        referrerpolicy="no-referrer"
      />
    );
  },
);

export default PluginFrame;
</script>
