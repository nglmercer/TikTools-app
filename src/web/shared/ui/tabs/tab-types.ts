import type { Component } from 'vue';
import type { SelectOption } from '../types.ts';

/** A tab with an optional leading icon and a small trailing count. */
export interface TabOption<T> extends SelectOption<T> {
  icon?: Component;
  /** Stable guided-tour target rendered as a data-guide attribute. */
  guide?: string;
  /** Shown after the label; omit (or leave undefined) to show nothing. */
  badge?: string | number;
}
