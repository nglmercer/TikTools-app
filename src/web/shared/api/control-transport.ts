/** Wire transport shared by native WebView and the local browser adapter. */
export interface ControlTransport {
  send(message: string): void;
  attach(receive: (message: string) => void, disconnected: (reason: string) => void, reconnected: () => void): void;
  detach(): void;
}
