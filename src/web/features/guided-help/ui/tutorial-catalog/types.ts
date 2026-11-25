import type { Component } from 'vue';
import type { LocalizedTutorialEntry, TutorialCategory } from '../../model/tutorial-catalog.ts';

export interface TutorialCategorySummary {
  id: TutorialCategory;
  label: string;
  icon: Component;
  count: number;
  entries: readonly LocalizedTutorialEntry[];
}

export type TutorialCatalogScreen = 'catalog' | 'category' | 'detail';
