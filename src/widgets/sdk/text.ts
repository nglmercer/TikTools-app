import { inject, type ComputedRef, type InjectionKey } from 'vue';
import type { WidgetStyle } from './template.ts';

export const textDefaults = {
  follow: { title: 'New follower', name: '{{name}}', handle: '{{username}}', message: 'just followed!' },
  share: { title: 'Shared', name: '{{name}}', handle: '{{username}}', message: 'shared the LIVE!' },
  subscribe: { title: 'New subscriber', name: '{{name}}', handle: '{{username}}', message: 'just subscribed!' },
  gift: { title: 'Gift received', streakTitle: 'Gift streak', name: '{{name}}', message: 'sent {{gift}}', count: '×{{count}}', diamonds: '{{diamonds}} diamonds' },
  chat: { name: '{{name}}', message: '{{message}}' },
} as const;
export type TextField = 'title' | 'streakTitle' | 'name' | 'handle' | 'message' | 'count' | 'diamonds';
export const textFields: TextField[] = ['title', 'streakTitle', 'name', 'handle', 'message', 'count', 'diamonds'];

/** Fields the builder keeps available without a remove control. */
export const coreTextFields: Record<keyof typeof textDefaults, TextField[]> = {
  follow: ['handle', 'message'],
  share: ['handle', 'message'],
  subscribe: ['handle', 'message'],
  // Diamonds are useful by default but optional: the builder can remove or
  // restore that line without changing the gift event data.
  gift: ['name', 'message', 'count'],
  chat: ['name', 'message'],
};

export function isHiddenField(design: WidgetStyle | undefined, field: TextField): boolean {
  return design?.hiddenText?.includes(field) ?? false;
}
export const widgetDesignKey: InjectionKey<ComputedRef<WidgetStyle>> = Symbol('widget-design');

/** One pass, plain text only: event values are never evaluated as templates or HTML. */
export function interpolateText(template: string, values: Record<string, string>): string {
  return template.replace(/\{\{\s*(\w+)\s*\}\}/g, (token, key: string) => Object.hasOwn(values, key) ? values[key]! : token);
}

export function useWidgetText(kind: keyof typeof textDefaults) {
  const design = inject(widgetDesignKey, undefined);
  return (field: TextField, event: { displayName: string; uniqueId?: string; giftName?: string; count?: number; totalDiamonds?: number; text?: string } | null) => {
    if (isHiddenField(design?.value, field)) return '';
    const defaults: Partial<Record<TextField, string>> = textDefaults[kind];
    return interpolateText(design?.value.text?.[field] ?? defaults[field] ?? '', {
      name: event?.displayName ?? '', username: event?.uniqueId ?? '', gift: event?.giftName ?? '',
      count: event?.count?.toLocaleString('en-US') ?? '', diamonds: event?.totalDiamonds?.toLocaleString('en-US') ?? '', message: event?.text ?? '',
    });
  };
}
