import { describe, expect, it } from 'vitest';
import { createActivityCounter, createRequestScope } from './request-scope.ts';

describe('request scope', () => {
  it('makes an earlier ticket stale when a newer one begins or the scope is invalidated', () => {
    const scope = createRequestScope();
    const first = scope.begin();
    const second = scope.begin();
    expect(first.isCurrent()).toBe(false);
    expect(second.isCurrent()).toBe(true);
    scope.invalidate();
    expect(second.isCurrent()).toBe(false);
  });

  it('shares the generation in force between concurrent callers until it is invalidated', () => {
    const scope = createRequestScope();
    const a = scope.current();
    const b = scope.current();
    expect(a.isCurrent() && b.isCurrent()).toBe(true);
    scope.invalidate();
    expect(a.isCurrent() || b.isCurrent()).toBe(false);
    expect(scope.current().isCurrent()).toBe(true);
  });

  it('keeps scopes independent', () => {
    const list = createRequestScope();
    const detail = createRequestScope();
    const listing = list.begin();
    detail.invalidate();
    expect(listing.isCurrent()).toBe(true);
  });
});

describe('activity counter', () => {
  it('stays active until every overlapping operation left, and leaving twice counts once', () => {
    const activity = createActivityCounter();
    const outer = activity.enter();
    const inner = activity.enter();
    inner(); inner();
    expect(activity.active.value).toBe(true);
    outer();
    expect(activity.active.value).toBe(false);
  });

  it('ignores operations that were running when the view was reset', () => {
    const activity = createActivityCounter();
    const stale = activity.enter();
    activity.reset();
    expect(activity.active.value).toBe(false);
    const fresh = activity.enter();
    stale();
    expect(activity.active.value).toBe(true);
    fresh();
    expect(activity.active.value).toBe(false);
  });
});
