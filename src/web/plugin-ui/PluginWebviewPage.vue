<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { Localized } from '../../shared/localized.ts';
import { i18nText, t, type Locale } from '../i18n.ts';
import { closePluginUi, isDesktopHost, openPluginUi } from './plugin-webview-host.ts';

type PluginWebviewPageProps = {
  locale: Locale;
  pluginId: string;
  pluginName: string;
  pageId: string;
  title: Localized;
};

/**
 * Generic launcher for a webview-mode plugin page.
 *
 * The plugin's compiled UI never loads in the main frontend: on desktop
 * the host opens it in an isolated native window, and on web the page
 * explains that the desktop app owns plugin views.
 */
export const PluginWebviewPage = defineVueComponent<PluginWebviewPageProps>(
  ['locale', 'pluginId', 'pluginName', 'pageId', 'title'],
  (props) => {
    const opened = ref(false);

    const open = (): void => {
      opened.value = openPluginUi(props.pluginId, props.pageId);
    };
    const close = (): void => {
      closePluginUi(props.pluginId, props.pageId);
      opened.value = false;
    };

    return () => {
      const locale: Locale = props.locale;
      const desktop = isDesktopHost();
      return (
        <div class="plg">
          <div class="plg-topbar">
            <div class="plg-topbar__text">
              <h2 class="plg-topbar__title">{i18nText(locale, props.title)}</h2>
              <span class="plg-topbar__subtitle">{props.pluginName}</span>
            </div>
          </div>
          <div class="plg-scroll">
            <div class="plg-stack">
              <div class="plg-card">
                <p class="plg-empty">
                  {desktop ? t(locale, 'pluginOpenUiHint') : t(locale, 'pluginDesktopUiOnly')}
                </p>
                {desktop && (
                  <div class="plg-chips">
                    <button type="button" class="plg-btn plg-btn--primary" onClick={open}>
                      {t(locale, 'pluginOpenUi')}
                    </button>
                    {opened.value && (
                      <button type="button" class="plg-btn" onClick={close}>
                        {t(locale, 'pluginCloseUi')}
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      );
    };
  },
);

export default PluginWebviewPage;
</script>
