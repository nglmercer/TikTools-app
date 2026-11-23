/** The two themes the frontend can paint. The desktop app stays dark by default. */
export type AppTheme = 'dark' | 'light';

export function isAppTheme(value: unknown): value is AppTheme {
  return value === 'dark' || value === 'light';
}

/** A standalone browser demo starts from the reader's OS preference. */
export function systemTheme(): AppTheme {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'dark';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

export function applyAppTheme(theme: AppTheme): void {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.style.colorScheme = theme;
}
