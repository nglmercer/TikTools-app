import { normalizeLocale, type Locale } from '../web/shared/i18n/supported-locales';
import { translatorFor, type Translate } from './messages';

/** Browser access the page needs; injected so the logic runs under test. */
export interface PairingEnvironment {
  readonly fetch: (input: string, init?: RequestInit) => Promise<Response>;
  readonly languages: readonly string[];
  /** The URL fragment as it was when the page loaded. */
  readonly hash: string;
  clearFragment(): void;
  goHome(): void;
}

type Kind = 'error' | 'info' | 'success';

interface RenewBody {
  readonly error?: string;
  readonly retryAfterSeconds?: unknown;
}

/** One-click terminal links carry the one-use code in the fragment, which browsers never send to the server. */
const JOIN_FRAGMENT = /#pair=([0-9]{6})(?:&|$)/;
const RENEW_COOLDOWN_SECONDS = 30;
const MAX_COOLDOWN_SECONDS = 300;

export function resolveLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const locale = normalizeLocale(language);
    if (locale) return locale;
  }
  return 'en';
}

function element<T extends HTMLElement>(doc: Document, id: string): T {
  const found = doc.getElementById(id);
  if (!found) throw new Error(`pairing page is missing #${id}`);
  return found as T;
}

export function mountPairing(doc: Document, env: PairingEnvironment): void {
  const locale = resolveLocale(env.languages);
  const t: Translate = translatorFor(locale);
  doc.documentElement.lang = locale;
  doc.title = t('documentTitle');
  element(doc, 'title').textContent = t('title');
  element(doc, 'intro').textContent = t('intro');
  element(doc, 'label-text').textContent = t('label');
  element(doc, 'connect').textContent = t('connect');

  const form = element<HTMLFormElement>(doc, 'pair');
  const input = element<HTMLInputElement>(doc, 'code');
  const result = element<HTMLElement>(doc, 'result');
  const renew = element<HTMLButtonElement>(doc, 'renew');
  renew.textContent = t('renew');

  let retryTimer: ReturnType<typeof setTimeout> | undefined;

  const show = (text: string, kind: Kind = 'error') => {
    result.textContent = text;
    result.dataset.kind = kind;
  };

  const cooldown = (seconds: number) => {
    clearTimeout(retryTimer);
    renew.disabled = true;
    const bounded = Math.min(MAX_COOLDOWN_SECONDS, Math.max(1, seconds));
    retryTimer = setTimeout(() => {
      renew.disabled = false;
    }, bounded * 1000);
  };

  const submitCode = async (code: string) => {
    input.value = '';
    try {
      const response = await env.fetch('/__local/pair', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ code }),
      });
      if (response.ok) {
        env.goHome();
        return;
      }
      show(t('invalidCode'));
    } catch {
      show(t('adapterUnavailable'));
    }
  };

  const requestCode = async () => {
    renew.disabled = true;
    input.value = '';
    show(t('requesting'), 'info');
    try {
      const response = await env.fetch('/__local/pair/renew', { method: 'POST' });
      const body = (await response.json()) as RenewBody;
      if (response.ok) {
        show(t('newCodePrinted'), 'success');
        cooldown(RENEW_COOLDOWN_SECONDS);
      } else if (body.error === 'cooldown') {
        const seconds = Number(body.retryAfterSeconds) || RENEW_COOLDOWN_SECONDS;
        show(t('waitSeconds', { seconds }), 'info');
        cooldown(seconds);
      } else if (body.error === 'pairing_locked') {
        show(t('lockedOut'));
      } else if (body.error === 'already_paired') {
        show(t('alreadyPaired'));
      } else {
        renew.disabled = false;
        show(t('renewFailed'));
      }
    } catch {
      renew.disabled = false;
      show(t('adapterUnavailable'));
    }
  };

  form.addEventListener('submit', (event) => {
    event.preventDefault();
    void submitCode(input.value);
  });

  const join = JOIN_FRAGMENT.exec(env.hash);
  if (join) {
    try {
      env.clearFragment();
    } catch {
      // The fragment is cosmetic; pairing proceeds either way.
    }
    show(t('signingIn'), 'info');
    void submitCode(join[1]!);
  }

  renew.addEventListener('click', () => {
    void requestCode();
  });
}
