import type { DocLocale } from '../locales.ts';
import type { ExperienceCopy } from './types.ts';
import en from './locales/en.ts';
import es from './locales/es.ts';
import pt from './locales/pt.ts';
import fr from './locales/fr.ts';
import zh from './locales/zh.ts';
import ja from './locales/ja.ts';

export const experienceCopy: Record<DocLocale, ExperienceCopy> = { en, es, pt, fr, zh, ja };
