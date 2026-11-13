import en from './en.ts';
import es from './es.ts';
import pt from './pt.ts';
import fr from './fr.ts';
import type { TourLocale } from '../model/types.ts';

type Widen<T> = T extends string ? string : { [K in keyof T]: Widen<T[K]> };
export type TourCatalog = Widen<typeof en>;
export const tourCatalogs: Record<TourLocale, TourCatalog> = { en, es, pt, fr };
