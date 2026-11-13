const marker = 'tiktools.demo.context:';
/** URL instance names can be copied into another tab. Bind storage to the
 * browsing context too, while retaining the identifier over an iframe reload. */
export function contextInstance(requested: string, windowName: string, nonce: () => string): string {
  const previous = windowName.startsWith(marker) ? windowName.slice(marker.length) : '';
  if (previous === requested && /^[a-zA-Z0-9_-]{1,64}$/.test(previous)) return previous;
  const value = `${requested.slice(0, 20)}-${nonce()}`;
  if (!/^[a-zA-Z0-9_-]{1,64}$/.test(value)) throw new Error('Invalid demo context identifier.');
  return value;
}
export const contextName = (instance: string) => marker + instance;
