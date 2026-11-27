import { userGuidePath } from '../../../docs/experience/landing/guides.ts';

const escapeRegExp = (text: string): string => text.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&');

/** Matches the end of a user-guide URL for a landing step or topic, optionally under a locale prefix such as `/es`. */
export const guideLink = (key: keyof typeof userGuidePath, prefix = ''): RegExp =>
  new RegExp(`${escapeRegExp(prefix)}/user/${escapeRegExp(userGuidePath[key])}$`);
