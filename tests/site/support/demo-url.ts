import { demoParam, demoSearchParams, type DemoParam } from '../../../src/web/demo/contract.ts';

type Values = Partial<Record<DemoParam, string | number | boolean | undefined>>;

/** Relative URL (to the site `baseURL`) of the standalone demo, e.g. `demoUrl({ scenario: 'feed', instance: 'x' })`. */
export function demoUrl(values: Values & { scenario?: string }): string {
  const params = demoSearchParams({ [demoParam.locale]: 'en', ...values });
  return `demo/?${params}`;
}

/** Unique, parseable instance id so parallel specs never share demo storage. */
export const instanceId = (...parts: Array<string | number>): string => parts.join('-').replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 64);
