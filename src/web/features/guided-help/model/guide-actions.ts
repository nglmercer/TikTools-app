/**
 * Semantic preparation operations registered by the page that owns a guide
 * scope. Tours must not infer application state by clicking whichever DOM
 * control happens to be visible; pages explicitly expose the safe operations
 * a guide may use (switch a tab, open an existing record, reveal a panel).
 */
export interface GuidePrepareContext {
  scope: Element | Document;
  target: string;
  /** Full semantic operation being executed, including any dynamic suffix. */
  action: string;
}

/** Return true only when the handler changed state toward the target. */
export type GuidePrepareHandler = (context: GuidePrepareContext) => boolean | Promise<boolean>;
export type GuidePrepareHandlers = Readonly<Record<string, GuidePrepareHandler>>;
interface GuidePrepareRegistration {
  owner?: Element;
  handler: GuidePrepareHandler;
}

const registrations = new Map<string, Map<string, GuidePrepareRegistration[]>>();

/** Register one scope owner's handlers; returns the exact undo operation. */
export function registerGuideActions(scope: string, handlers: GuidePrepareHandlers, owner?: Element): () => void {
  const scoped = registrations.get(scope) ?? new Map<string, GuidePrepareRegistration[]>();
  const added: Array<[string, GuidePrepareRegistration]> = [];
  for (const [action, handler] of Object.entries(handlers)) {
    const registration = { ...(owner ? { owner } : {}), handler };
    const entries = scoped.get(action) ?? [];
    if (entries.some((entry) => entry.owner === owner)) throw new Error(`Duplicate guide preparation ${scope}.${action}`);
    entries.push(registration);
    scoped.set(action, entries);
    added.push([action, registration]);
  }
  registrations.set(scope, scoped);
  return () => {
    const current = registrations.get(scope);
    if (!current) return;
    for (const [action, registration] of added) {
      const entries = current.get(action)?.filter((entry) => entry !== registration);
      if (entries?.length) current.set(action, entries);
      else current.delete(action);
    }
    if (!current.size) registrations.delete(scope);
  };
}

function candidates(scopeName: string | undefined, action: string, scope?: Element | Document): GuidePrepareRegistration[] {
  if (!scopeName) return [];
  const scoped = registrations.get(scopeName);
  const exact = scoped?.get(action) ?? [];
  // Dynamic plugin action types cannot be enumerated when the page mounts. A
  // scope owner may explicitly register a prefix handler such as
  // `automation.createActionType:*`; exact registrations always win.
  const wildcard = exact.length ? [] : [...(scoped?.entries() ?? [])]
    .filter(([name]) => name.endsWith(':*') && action.startsWith(name.slice(0, -1)))
    .flatMap(([, entries]) => entries);
  const entries = [...exact, ...wildcard];
  if (!(scope instanceof Element)) return [...entries].reverse();
  const owned = entries.filter((entry) => entry.owner && (entry.owner === scope || scope.contains(entry.owner)));
  return [...(owned.length ? owned : entries)].reverse();
}

/** Whether a mounted scope owner explicitly exposes one operation. */
export function hasGuideAction(scopeName: string | undefined, action: string, scope?: Element | Document): boolean {
  return candidates(scopeName, action, scope).length > 0;
}

/** Run one named operation if the mounted scope owner provides it. */
export async function runGuideAction(scope: Element | Document, scopeName: string | undefined, action: string, target: string): Promise<boolean> {
  const handler = candidates(scopeName, action, scope)[0]?.handler;
  if (!handler) return false;
  try { return await handler({ scope, target, action }) === true; }
  catch { return false; }
}
