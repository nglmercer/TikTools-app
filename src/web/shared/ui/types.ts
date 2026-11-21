import type { Component } from 'vue';

export type ControlSize = 'sm' | 'md';

export interface SelectOption<T> {
  value: T;
  label: string;
  disabled?: boolean;
  icon?: Component;
}
