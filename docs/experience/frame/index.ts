import type { DocLocale } from '../../locales.ts';
import type { FrameCopy } from './types.ts';
import en from './en.ts';
import es from './es.ts';
import pt from './pt.ts';
import fr from './fr.ts';
import zh from './zh.ts';
import ja from './ja.ts';
export const frameCopy: Record<DocLocale, FrameCopy> = { en, es, pt, fr, zh, ja };
