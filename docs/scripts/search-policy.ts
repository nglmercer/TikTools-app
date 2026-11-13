import type { DefaultTheme } from 'vitepress';
export function indexableDocument(frontmatter: Record<string, unknown>): boolean {
  return frontmatter.search !== false && frontmatter.contentStatus === 'reviewed' && frontmatter.featureStatus !== 'historical' && frontmatter.audience !== 'internal';
}
export const renderDocSearch: NonNullable<DefaultTheme.LocalSearchOptions['_render']> = (source, env, md) => {
  const html = md.render(source, env);
  return indexableDocument(env.frontmatter ?? {}) ? html : '';
};
