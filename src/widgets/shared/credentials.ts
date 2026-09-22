import { DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT } from './config.ts';

export interface CredentialInput {
  /** Location hash such as `#token=ttw_…&host=127.0.0.1&port=17452`. */
  hash?: string;
  /** Fallbacks when the fragment omits host/port (page origin by default). */
  defaultHost?: string;
  defaultPort?: number;
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
  private readonly defaultHost: string;
  private readonly defaultPort: number;

  constructor(input: CredentialInput = {}) {
    this.params = new URLSearchParams((input.hash ?? '').replace(/^#/, ''));
    this.defaultHost =
      input.defaultHost && input.defaultHost.trim() !== ''
        ? input.defaultHost
        : DEFAULT_GATEWAY_HOST;
    this.defaultPort =
      input.defaultPort !== undefined &&
      Number.isInteger(input.defaultPort) &&
      input.defaultPort >= 1 &&
      input.defaultPort <= 65535
        ? input.defaultPort
        : DEFAULT_GATEWAY_PORT;
  }

  /**
   * Reads credentials from the live page URL. The token comes from the
   * fragment only; host/port fall back to the serving page origin so the
   * widget always talks back to the gateway that served it (custom ports
   * included), unless the fragment explicitly overrides them.
   */
  static fromWindow(): CredentialProvider {
    if (typeof window === 'undefined') return new CredentialProvider({});
    const port = Number.parseInt(window.location.port, 10);
    return new CredentialProvider({
      hash: window.location.hash,
      defaultHost: window.location.hostname || undefined,
      defaultPort:
        Number.isInteger(port) && port >= 1 && port <= 65535 ? port : undefined,
    });
  }

  /** Widget credential from `#token=…`, or null when absent. */
  get token(): string | null {
    const token = this.params.get('token');
    return token && token.trim() !== '' ? token : null;
  }

  get host(): string {
    const host = this.params.get('host');
    return host && host.trim() !== '' ? host : this.defaultHost;
  }

  get port(): number {
    const parsed = Number.parseInt(this.params.get('port') ?? '', 10);
    return Number.isInteger(parsed) && parsed >= 1 && parsed <= 65535
      ? parsed
      : this.defaultPort;
  }
}
