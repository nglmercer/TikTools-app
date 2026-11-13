/** Explicit simulation clock; native/browser host entries retain wall-clock time. */
let clock: (() => number) | undefined;
export const appNow = (): number => clock?.() ?? Date.now();
export function configureAppClock(next: (() => number) | undefined): void { clock = next; }
