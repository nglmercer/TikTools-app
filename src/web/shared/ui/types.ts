import type { Component } from 'vue';

export type ControlSize = 'sm' | 'md';

export interface SelectOption<T> {
  value: T;
  label: string;
  disabled?: boolean;
  icon?: Component;
  /** Tints the option icon — for pickers whose options double as a color legend. */
  color?: string;
}
