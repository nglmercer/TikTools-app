/**
 * Isolated gateway credential handling. Widgets read the token from the URL
 * fragment (`#token=...`), an injected page global, or loopback-only local
 * storage — never from the query string, which leaks into server logs.
 *
 * This is the interim mechanism until TikTools issues scoped widget
 * credentials; all reads flow through this provider so that swap stays local.
 */

import { DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT } from './config.ts';

export interface CredentialInputs {
  /** Raw `location.hash`, including the leading `#`. */
  hash?: string;
  /** Pre-injected token (TikTools preview pages). */
  injectedToken?: unknown;
  /** Optional storage for a previously saved token (dev convenience). */
  storage?: Pick<Storage, 'getItem' | 'setItem'> | null;
}

const STORAGE_KEY = 'tiktools.gateway.token';

function readFragmentParams(hash: string | undefined): URLSearchParams {
  if (!hash) return new URLSearchParams();
  return new URLSearchParams(hash.startsWith('#') ? hash.slice(1) : hash);
}

function readPort(params: URLSearchParams): number | undefined {
  const raw = params.get('port');
  if (raw === null || raw.trim() === '') return undefined;
  const port = Number.parseInt(raw, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return undefined;
  return port;
}

export class CredentialProvider {
  private readonly params: URLSearchParams;
  private readonly injectedToken: string | null;
  private readonly storage: Pick<Storage, 'getItem' | 'setItem'> | null;

  constructor(inputs: CredentialInputs = {}) {
    this.params = readFragmentParams(inputs.hash);
    this.injectedToken =
      typeof inputs.injectedToken === 'string' && inputs.injectedToken.trim() !== ''
        ? inputs.injectedToken
        : null;
    this.storage = inputs.storage ?? null;
  }

  static fromWindow(): CredentialProvider {
    if (typeof window === 'undefined') return new CredentialProvider();
    const injected = (window as unknown as Record<string, unknown>)['__TIKTOOLS_GATEWAY_TOKEN__'];
    let storage: Storage | null;
    try {
      storage = window.localStorage ?? null;
    } catch {
      storage = null;
    }
    return new CredentialProvider({ hash: window.location.hash, injectedToken: injected, storage });
  }

  get token(): string | null {
    const fragment = this.params.get('token')?.trim();
    if (fragment) return fragment;
    if (this.injectedToken) return this.injectedToken;
    try {
      const stored = this.storage?.getItem(STORAGE_KEY)?.trim();
      if (stored) return stored;
    } catch {
      // Storage may be unavailable (opaque origin); fall through.
    }
    return null;
  }

  get host(): string {
    const host = this.params.get('host')?.trim();
    return host ? host : DEFAULT_GATEWAY_HOST;
  }

  get port(): number {
    return readPort(this.params) ?? DEFAULT_GATEWAY_PORT;
  }

  /** Persists a fragment-supplied token for page reloads (best effort). */
  rememberFragmentToken(): void {
    const fragment = this.params.get('token')?.trim();
    if (!fragment || !this.storage) return;
    try {
      this.storage.setItem(STORAGE_KEY, fragment);
    } catch {
      // Ignore: storage is a convenience, not a requirement.
    }
  }
}
