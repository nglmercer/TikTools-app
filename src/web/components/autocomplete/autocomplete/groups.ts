/* ------------------------------------------------------------------ */
/* Template groups (S11): neutral module, zero imports. rows.ts and    */
/* icons.ts both import from here; importing from either of them here  */
/* would reintroduce the rows <-> icons cycle (see                    */
/* ../autocomplete-import-graph.test.ts).                              */
/* ------------------------------------------------------------------ */

export type TemplateGroupName = 'User' | 'Message' | 'Text Intelligence';

/** Section order for grouped template rows (S11). */
export const TEMPLATE_GROUP_ORDER: readonly TemplateGroupName[] = ['User', 'Message', 'Text Intelligence'];

/**
 * Group one template path. Identity paths → User, Text Intelligence
 * views → Text Intelligence, everything else → Message.
 */
export function groupForTemplatePath(path: string): TemplateGroupName {
  const normalized = path.trim().toLowerCase();
  if (normalized === 'event.user' || normalized.startsWith('event.user.') || normalized.startsWith('event.intel.user.')) {
    return 'User';
  }
  if (normalized === 'event.intel' || normalized.startsWith('event.intel.')) return 'Text Intelligence';
  return 'Message';
}
