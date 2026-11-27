import { testMessage } from '../../../src/web/shared/i18n/test-message.ts';

/** English UI copy for assertions, resolved from the real catalog so specs never restate it. */
export const t = (key: string): string => testMessage('en', key);
