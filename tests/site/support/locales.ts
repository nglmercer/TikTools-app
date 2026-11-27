import { docLocales } from '../../../docs/locales.ts';
import { demoLocales } from '../../../src/web/demo/scenarios.ts';

export { docLocales, demoLocales };
/** Locales whose scripts need dedicated font and wrapping checks. */
export const cjkLocales = ['zh', 'ja'] as const;
