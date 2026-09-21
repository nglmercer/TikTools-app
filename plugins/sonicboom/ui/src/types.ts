/**
 * Package-local UI types. The plugin UI never imports the main frontend
 * (`src/web`) or automation: these structural mirrors are the only
 * contract surface, and they are exercised against the real host by the
 * isolation tests (main-app Playwright suite) and this package's suite.
 */

/** One option row from an option source (mirrors the host shape). */
export interface ActionOptionItem {
  value: string;
  label: string;
}

/** Log entry rendered by the voice tester (mirrors shared TTS types). */
export interface UiLogEntry {
  id: number;
  at: number;
  ok: boolean;
  source: string;
  text: string;
  voice: string;
  summary: string;
}
