import { DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT } from './config.ts';

export interface CredentialInput {
  /** Location hash such as `#token=ttw_…&host=127.0.0.1&port=17452`. */
  hash?: string;
}

/**
 * Gateway credential lookup: URL fragment -> memory -> WebSocket auth.
 *
 * OBS persists the Browser Source URL itself, so the widget never writes
 * the credential anywhere else: no localStorage, no cookies, no query
 * parameters. The token authenticates one WebSocket handshake and then
 * lives only in this provider's memory.
 */
export class CredentialProvider {
  private readonly params: URLSearchParams;

  constructor(input: CredentialInput = {}) {
    this.params = new URLSearchParams((input.hash ?? '').replace(/^#/, ''));
  }

  /** Widget credential from `#token=…`, or null when absent. */
  get token(): string | null {
    const token = this.params.get('token');
    return token && token.trim() !== '' ? token : null;
  }

  get host(): string {
    const host = this.params.get('host');
    return host && host.trim() !== '' ? host : DEFAULT_GATEWAY_HOST;
  }

  get port(): number {
    const parsed = Number.parseInt(this.params.get('port') ?? '', 10);
    return Number.isInteger(parsed) && parsed >= 1 && parsed <= 65535
      ? parsed
      : DEFAULT_GATEWAY_PORT;
  }
}
