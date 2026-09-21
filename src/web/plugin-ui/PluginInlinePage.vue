<script lang="tsx">
import { computed } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { Localized } from '../../shared/localized.ts';
import { i18nText, t, type Locale } from '../i18n.ts';
import { PluginFrame } from './PluginFrame.vue';
import {
  openPluginUi,
  pluginUiUrl,
  type BrokerBackend,
} from './plugin-webview-host.ts';

type PluginInlinePageProps = {
  locale: Locale;
  pluginId: string;
  pluginName: string;
  pageId: string;
  title: Localized;
  /** Manifest UI entry (e.g. `ui/dist/index.html`); only the file name travels. */
  entry: string;
  backend: BrokerBackend;
  subscribeTopic?: (topic: string, listener: (data: unknown) => void) => () => void;
  onError?: (message: string) => void;
};

/**
 * Inline tab body for a webview-mode plugin page.
 *
 * The plugin's compiled UI renders inside the tab in a sandboxed frame
 * at an opaque origin — separate document, separate storage, no access
 * to the host — and talks to the host only through the versioned broker
 * envelope answered by `backend`. Plugin JS is never imported into the
 * main Vue runtime. A pop-out action opens the same page in an isolated
 * native window on desktop.
 */
export const PluginInlinePage = defineVueComponent<PluginInlinePageProps>(
  ['locale', 'pluginId', 'pluginName', 'pageId', 'title', 'entry', 'backend', 'subscribeTopic', 'onError'],
  (props) => {
    const src = computed(() => pluginUiUrl(props.pluginId, props.entry, props.pageId));
    const popOut = (): void => {
      openPluginUi(props.pluginId, props.pageId);
    };

    return () => {
      const locale: Locale = props.locale;
      const frameSrc = src.value;
      return (
        <div class="plg">
          <div class="plg-topbar">
            <div class="plg-topbar__text">
              <h2 class="plg-topbar__title">{i18nText(locale, props.title)}</h2>
              <span class="plg-topbar__subtitle">{props.pluginName}</span>
            </div>
            {frameSrc && (
              <div class="plg-topbar__actions">
                <button type="button" class="plg-btn plg-btn--ghost plg-btn--sm" onClick={popOut}>
                  {t(locale, 'pluginOpenSeparate')}
                </button>
              </div>
            )}
          </div>
          <div class="plg-scroll">
            <div class="plg-stack">
              {frameSrc ? (
                <PluginFrame
                  locale={locale}
                  pluginId={props.pluginId}
                  src={frameSrc}
                  backend={props.backend}
                  subscribeTopic={props.subscribeTopic}
                  onError={props.onError}
                />
              ) : (
                <div class="plg-card">
                  <p class="plg-empty">{t(locale, 'pluginDesktopUiOnly')}</p>
                </div>
              )}
            </div>
          </div>
        </div>
      );
    };
  },
);

export default PluginInlinePage;
</script>
