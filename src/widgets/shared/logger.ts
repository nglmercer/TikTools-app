/**
 * Minimal scoped logger for widgets. Development keeps debug/info; production
 * keeps warn/error. Callers must never pass tokens or cookies here.
 */

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const ORDER: Record<LogLevel, number> = { debug: 0, info: 1, warn: 2, error: 3 };

export interface Logger {
  debug: (...args: unknown[]) => void;
  info: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
}

function defaultLevel(): LogLevel {
  // Vite statically replaces import.meta.env.DEV (true in dev, false in
  // builds); other hosts (bun tests, plain browsers) leave it undefined,
  // which falls back to the production level.
  return import.meta.env.DEV ? 'debug' : 'warn';
}

export function createLogger(scope: string, level?: LogLevel): Logger {
  const threshold = ORDER[level ?? defaultLevel()];
  const emit = (entry: LogLevel, consoleMethod: 'debug' | 'info' | 'warn' | 'error') => {
    return (...args: unknown[]): void => {
      if (ORDER[entry] < threshold) return;
      console[consoleMethod](`[tiktools:${scope}]`, ...args);
    };
  };
  return {
    debug: emit('debug', 'debug'),
    info: emit('info', 'info'),
    warn: emit('warn', 'warn'),
    error: emit('error', 'error'),
  };
}
