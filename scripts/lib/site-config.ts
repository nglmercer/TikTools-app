/** Shared deployment path for documentation, demo, assembly and previews. */
export function normalizeSiteBase(value: string): string {
  if (!value.startsWith('/') || value.includes('\\') || value.includes('?') || value.includes('#')) throw new Error('Site base must be an absolute URL path.');
  const parts = value.split('/').filter(Boolean);
  if (parts.some((part) => !/^[a-zA-Z0-9._~-]+$/.test(part) || part === '.' || part === '..')) throw new Error('Invalid site base segment.');
  return parts.length ? `/${parts.join('/')}/` : '/';
}

export function siteConfig(env: Record<string, string | undefined> = process.env) {
  const base = normalizeSiteBase(env.TIKTOOLS_SITE_BASE ?? '/TikTools-app/');
  const origin = new URL(env.TIKTOOLS_SITE_ORIGIN ?? 'https://nglmercer.github.io');
  if (origin.protocol !== 'https:' && origin.hostname !== '127.0.0.1' && origin.hostname !== 'localhost') throw new Error('Published site origin must use HTTPS.');
  if (origin.pathname !== '/' || origin.search || origin.hash || origin.username || origin.password) throw new Error('Site origin must not include a path or credentials.');
  return { base, origin: origin.origin, url: new URL(base, origin).href };
}
