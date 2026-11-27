/** A real `MediaQueryList` for tests that replace `window.matchMedia`, so no fake has to be cast to the DOM type. */
export class FakeMediaQuery extends EventTarget implements MediaQueryList {
  onchange: ((this: MediaQueryList, event: MediaQueryListEvent) => unknown) | null = null;

  constructor(readonly media: string, readonly matches: boolean) {
    super();
  }

  addListener(): void {}

  removeListener(): void {}
}
