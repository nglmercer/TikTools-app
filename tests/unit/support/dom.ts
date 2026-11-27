/** Typed DOM lookups for tests: they narrow with `instanceof`, so no `as HTMLInputElement` casts are needed. */
type ElementConstructor<T extends Element> = abstract new (...args: never[]) => T;

/** The first element matching `selector` under `root`, narrowed to `kind`; throws a readable error otherwise. */
export function expectElement<T extends Element>(root: ParentNode, selector: string, kind: ElementConstructor<T>): T {
  const found = root.querySelector(selector);
  if (!(found instanceof kind)) throw new Error(`Expected "${selector}" to match a ${kind.name}, got ${found ? found.constructor.name : 'nothing'}`);
  return found;
}

/** Narrow a node you already hold (a wrapper's `.element`, an event target) to `kind`. */
export function toElement<T extends Element>(node: unknown, kind: ElementConstructor<T>): T {
  if (!(node instanceof kind)) {
    const got = node === null ? 'null' : typeof node === 'object' ? node.constructor.name : typeof node;
    throw new Error(`Expected a ${kind.name}, got ${got}`);
  }
  return node;
}

/** A real `DOMRectList` for tests that stub `getClientRects`, so no array has to be cast to the DOM type. */
class FakeDOMRectList implements DOMRectList {
  [index: number]: DOMRect;
  readonly length: number;

  constructor(rects: readonly DOMRect[]) {
    this.length = rects.length;
    rects.forEach((rect, index) => {
      this[index] = rect;
    });
  }

  item(index: number): DOMRect | null {
    return this[index] ?? null;
  }

  *[Symbol.iterator](): ArrayIterator<DOMRect> {
    for (let index = 0; index < this.length; index++) yield this[index]!;
  }
}

export const rectList = (...rects: DOMRect[]): DOMRectList => new FakeDOMRectList(rects);
