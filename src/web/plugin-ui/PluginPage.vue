<script lang="tsx">
import { onMounted, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import { optionFields } from '../../automation/plugins/declarative.ts';
import { collectOptionSources, type PluginUiNode, type PluginUiPage } from '../../plugin-ui/index.ts';
import { i18nText } from '../i18n.ts';
import { PluginNode } from './PluginNode.vue';
import type { PluginUiContext } from './PluginUiContext.ts';

type PluginPageProps = {
  page: PluginUiPage;
  context: PluginUiContext;
};

function formNodes(body: PluginUiNode): PluginUiNode[] {
  if (body.type === 'stack' || body.type === 'card') {
    return body.children.flatMap((child) => formNodes(child));
  }
  return body.type === 'form' ? [body] : [];
}

/**
 * Generic host-rendered plugin page. Renders any normalized `PluginUiPage`
 * through `<PluginNode>` with a single stable context.
 *
 * This module is domain-free: it never imports TTS (or any domain) UI.
 * Domain node types resolve through `context.customNodes`, bound by the
 * composition root to host-owned components.
 */
export const PluginPage = defineVueComponent<PluginPageProps>(['page', 'context'], (props) => {
  const requestSources = (): void => {
    const context = props.context;
    if (!context.settings.state) context.settings.get();
    for (const source of collectOptionSources(props.page.body)) context.options.get(source);
    for (const node of formNodes(props.page.body)) {
      if (node.type !== 'form') continue;
      for (const field of optionFields(node.uiHints ?? context.settings.state?.uiHints)) {
        context.options.get(field.source);
      }
    }
  };

  const resetPageState = (): void => {
    props.context.formDrafts.clear();
  };

  onMounted(requestSources);
  watch(() => props.page, () => {
    resetPageState();
    requestSources();
  });
  watch(
    () => props.context.settings.state?.uiHints,
    () => {
      const context = props.context;
      for (const node of formNodes(props.page.body)) {
        if (node.type !== 'form') continue;
        for (const field of optionFields(node.uiHints ?? context.settings.state?.uiHints)) {
          context.options.get(field.source);
        }
      }
    },
  );

  return () => {
    const context = props.context;
    const locale = context.locale;
    const body = props.page.body;
    const topNodes = body.type === 'stack' || body.type === 'card' ? body.children : [body];
    return (
      <div class="plg">
        <div class="plg-topbar">
          <div class="plg-topbar__text">
            <h2 class="plg-topbar__title">{i18nText(locale, props.page.title)}</h2>
            <span class="plg-topbar__subtitle">{context.pluginName}</span>
          </div>
        </div>
        <div class="plg-scroll">
          <div class="plg-stack">
            {topNodes.map((node, index) => (
              <PluginNode key={index} node={node} context={context} nodeKey={`s${index}`} />
            ))}
          </div>
        </div>
      </div>
    );
  };
});

export default PluginPage;
</script>
