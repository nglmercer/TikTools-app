export type {
  AutocompleteItem,
  AutocompleteKind,
  AutocompleteSource,
  GenericSuggestion,
  ScoredSuggestion,
} from './autocomplete.ts';

export type AutocompleteToken = {
  start: number;
  query: string;
  inside: boolean;
};

export type AutocompleteRow<T = unknown> = {
  key?: string;
  item: import('./autocomplete.ts').AutocompleteItem;
  ranges: Array<{ start: number; end: number }>;
  meta?: T;
};
