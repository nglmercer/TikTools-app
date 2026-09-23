import { describe, expect, test } from 'bun:test';

import { AlertQueue, RecentEventIds } from './alert-queue.ts';
import { computeReconnectDelay } from './reconnect.ts';

describe('alert queue', () => {
  test('preserves FIFO ordering', () => {
    const queue = new AlertQueue<string>();
    queue.enqueue('a');
    queue.enqueue('b');
    expect(queue.dequeue()).toBe('a');
    expect(queue.dequeue()).toBe('b');
    expect(queue.dequeue()).toBeUndefined();
  });

  test('drop-oldest discards the stalest alert when full', () => {
    const queue = new AlertQueue<string>(2, 'drop-oldest');
    expect(queue.enqueue('a')).toBe(true);
    expect(queue.enqueue('b')).toBe(true);
    expect(queue.enqueue('c')).toBe(true);
    expect(queue.size).toBe(2);
    expect(queue.dequeue()).toBe('b');
    expect(queue.dequeue()).toBe('c');
  });

  test('drop-newest keeps existing alerts when full', () => {
    const queue = new AlertQueue<string>(2, 'drop-newest');
    queue.enqueue('a');
    queue.enqueue('b');
    expect(queue.enqueue('c')).toBe(false);
    expect(queue.dequeue()).toBe('a');
    expect(queue.dequeue()).toBe('b');
  });

  test('clear empties the queue', () => {
    const queue = new AlertQueue<string>();
    queue.enqueue('a');
    queue.clear();
    expect(queue.size).toBe(0);
    expect(queue.dequeue()).toBeUndefined();
  });
});

describe('recent event ids', () => {
  test('rejects duplicates and evicts oldest beyond capacity', () => {
    const recent = new RecentEventIds(2);
    expect(recent.add('a')).toBe(true);
    expect(recent.add('a')).toBe(false);
    expect(recent.add('b')).toBe(true);
    expect(recent.add('c')).toBe(true);
    expect(recent.size).toBe(2);
    expect(recent.has('b')).toBe(true);
    expect(recent.has('c')).toBe(true);
    expect(recent.has('a')).toBe(false);
  });
});

describe('reconnect delay', () => {
  test('doubles from 500ms to a 10s ceiling without jitter', () => {
    const delays = [0, 1, 2, 3, 4, 5, 6, 10].map((attempt) =>
      computeReconnectDelay(attempt, { random: () => 0 }),
    );
    expect(delays).toEqual([500, 1000, 2000, 4000, 8000, 10000, 10000, 10000]);
  });

  test('adds bounded jitter', () => {
    const base = computeReconnectDelay(1, { random: () => 0 });
    const jittered = computeReconnectDelay(1, { random: () => 1 });
    expect(jittered - base).toBeLessThanOrEqual(250);
    expect(jittered).toBeGreaterThanOrEqual(base);
  });

  test('clamps negative attempts', () => {
    expect(computeReconnectDelay(-3, { random: () => 0 })).toBe(500);
  });
});
