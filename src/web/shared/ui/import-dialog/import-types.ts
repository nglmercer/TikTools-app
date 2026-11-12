import type { Component } from 'vue';

/** One source of an import: a local list, the Cloud, or a manual file/paste. */
export interface ImportTab {
  value: string;
  label: string;
  icon?: Component;
  badge?: string | number;
  disabled?: boolean;
}

/** One parsed item waiting for the user to confirm the import. */
export interface StagedImportItem {
  key: string;
  label: string;
  detail?: string;
}

/** Value of the tab whose body is the file/paste drop zone. */
export const MANUAL_IMPORT_TAB = 'manual';

/** Texts of the drop zone and the staged list; any omitted one falls back to `importDialog.*`. */
export interface ImportDialogLabels {
  dropTitle?: string;
  dropHint?: string;
  browse?: string;
  paste?: string;
  pasteBlocked?: string;
  clipboard?: string;
  unreadable?: string;
  tooLarge?: string;
  staged?: string;
  clear?: string;
  run?: string;
  close?: string;
}
