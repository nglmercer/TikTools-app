import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { applyConditionTemplate, type ConditionTemplate } from './condition-templates.ts';
import { liveGoalTemplates } from './live-goal-templates.ts';
import { moderationConditionTemplates } from './moderation-templates.ts';
import { matchesAllFilters, matchesFilter } from './testing/filter-matcher.ts';

/**
 * The corpus is shared with the Rust matcher (`filters.rs` `corpus_tests`).
 * Rust is the contract; this suite proves the test-support mirror agrees case
 * by case, and that the template builders still emit the stored filters the
 * production cases evaluate.
 */
interface CorpusCase {
  label: string;
  event: string | Record<string, unknown>;
  matches: boolean;
  filter?: object;
  filters?: unknown;
  template?: string;
  answers?: Record<string, string>;
}

interface Corpus {
  events: Record<string, Record<string, unknown>>;
  filterCases: CorpusCase[];
  recordCases: CorpusCase[];
  productionCases: CorpusCase[];
}

const corpus = JSON.parse(
  readFileSync(resolve(import.meta.dir, '../../../tests/fixtures/automation-filters/filter-corpus.json'), 'utf8'),
) as Corpus;

function eventOf(entry: CorpusCase): unknown {
  if (typeof entry.event !== 'string') return entry.event;
  const named = corpus.events[entry.event];
  if (!named) throw new Error(`unknown corpus event ${entry.event}`);
  return named;
}

function templateById(id: string): ConditionTemplate {
  const found = [...liveGoalTemplates(), ...moderationConditionTemplates()].find((template) => template.id === id);
  if (!found) throw new Error(`unknown template ${id}`);
  return found;
}

describe('shared filter corpus', () => {
  test('covers every section', () => {
    expect(corpus.filterCases.length).toBeGreaterThan(100);
    expect(corpus.recordCases.length).toBeGreaterThan(0);
    expect(corpus.productionCases.length).toBeGreaterThan(0);
    const operators = new Set(corpus.filterCases.map((entry) => (entry.filter as { operator?: string }).operator));
    for (const operator of ['gte', 'gt', 'lte', 'lt', 'eq', 'neq', 'contains', 'starts-with', 'in', 'is-true', 'is-false']) {
      expect(operators.has(operator)).toBe(true);
    }
  });

  test('filter cases agree with the host matcher', () => {
    const failures = corpus.filterCases
      .filter((entry) => matchesFilter(entry.filter as object, eventOf(entry)) !== entry.matches)
      .map((entry) => entry.label);
    expect(failures).toEqual([]);
  });

  test('record cases combine filters with AND like the host', () => {
    const failures = corpus.recordCases
      .filter((entry) => matchesAllFilters(entry.filters, eventOf(entry)) !== entry.matches)
      .map((entry) => entry.label);
    expect(failures).toEqual([]);
  });

  test('production cases agree with the host matcher', () => {
    const failures = corpus.productionCases
      .filter((entry) => matchesAllFilters(entry.filters, eventOf(entry)) !== entry.matches)
      .map((entry) => entry.label);
    expect(failures).toEqual([]);
  });

  test('production filters are exactly what the template builders emit', () => {
    const templated = corpus.productionCases.filter((entry) => entry.template !== undefined);
    expect(templated.length).toBeGreaterThan(0);
    for (const entry of templated) {
      expect(applyConditionTemplate(templateById(entry.template ?? ''), entry.answers ?? {})).toEqual(
        entry.filters as never,
      );
    }
  });
});
