import type { MarkdownOptions } from 'vitepress';

const marker = 'contract-schema';
const escapeHtml = (text: string) => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');

/**
 * The generated RPC reference embeds two JSON schemas for each of ~300 methods
 * in every locale. Shiki-tokenizing those megabytes of identical JSON dominated
 * the VitePress build (and produced multi-megabyte page chunks). Fences tagged
 * `contract-schema` are emitted as plain, escaped `<pre>` blocks instead, while
 * every other code block keeps the normal VitePress highlighting.
 */
export const plainSchemaShikiSetup: NonNullable<MarkdownOptions['shikiSetup']> = (shiki) => {
  const codeToHtml = shiki.codeToHtml.bind(shiki);
  shiki.codeToHtml = ((code: string, options: { meta?: { __raw?: string } }) => {
    if (String(options?.meta?.__raw ?? '').includes(marker)) {
      return `<pre v-pre><code class="language-json">${escapeHtml(code)}</code></pre>`;
    }
    return codeToHtml(code, options as Parameters<typeof shiki.codeToHtml>[1]);
  }) as unknown as typeof shiki.codeToHtml;
};
