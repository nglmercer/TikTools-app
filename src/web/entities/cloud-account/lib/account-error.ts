type Translate = (key: string) => string;

/**
 * Maps raw Cloud account failures to short user-facing text. The raw
 * message stays available in Settings › Advanced for diagnostics.
 */
export function friendlyAccountError(raw: string | null | undefined, t: Translate): string | null {
  if (!raw) return null;
  const text = raw.toLowerCase();
  if (text.includes('cancelled') || text.includes('canceled')) return null;
  if (text.includes('expired')) return t('account.errors.expired');
  if (text.includes('credential store')) return t('account.errors.keychain');
  if (
    /error sending request|connect|timed out|timeout|dns|network|unreachable|503|502|504/.test(text)
  ) {
    return t('account.errors.unreachable');
  }
  return t('account.errors.failed');
}
