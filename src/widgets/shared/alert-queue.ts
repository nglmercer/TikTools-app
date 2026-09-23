/**
 * Bounded alert queue plus bounded duplicate-protection cache. Neither
 * structure is allowed to grow without limit during event bursts.
 */

export type QueueDropPolicy = 'drop-oldest' | 'drop-newest';

export class AlertQueue<T> {
  private readonly items: T[] = [];

  constructor(
    private readonly capacity: number = 100,
    private readonly policy: QueueDropPolicy = 'drop-oldest',
  ) {}

  get size(): number {
    return this.items.length;
  }

  /**
   * Appends an alert. When full, either the oldest alert is discarded to
   * make room (default: bursts surface the freshest alerts) or the incoming
   * alert is dropped. Returns false only when the incoming alert is dropped.
   */
  enqueue(alert: T): boolean {
    if (this.items.length >= this.capacity) {
      if (this.policy === 'drop-newest') return false;
      this.items.shift();
    }
    this.items.push(alert);
    return true;
  }

  dequeue(): T | undefined {
    return this.items.shift();
  }

  peek(): T | undefined {
    return this.items[0];
  }

  clear(): void {
    this.items.length = 0;
  }
}

/**
 * Bounded set of recently processed event ids. `add` returns false when the
 * id was already seen (the caller must ignore the event). Oldest ids are
 * evicted first once capacity is reached.
 */
export class RecentEventIds {
  private readonly ids = new Map<string, true>();

  constructor(private readonly capacity: number = 1000) {}

  get size(): number {
    return this.ids.size;
  }

  has(id: string): boolean {
    return this.ids.has(id);
  }

  add(id: string): boolean {
    if (this.ids.has(id)) return false;
    this.ids.set(id, true);
    while (this.ids.size > this.capacity) {
      const oldest = this.ids.keys().next().value as string | undefined;
      if (oldest === undefined) break;
      this.ids.delete(oldest);
    }
    return true;
  }

  clear(): void {
    this.ids.clear();
  }
}
