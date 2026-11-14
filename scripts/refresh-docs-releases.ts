import { writeFile } from 'node:fs/promises';

// Explicit refresh: builds and documentation visitors never depend on GitHub API availability.
const repository = 'nglmercer/TikTools-app';
const source = `https://api.github.com/repos/${repository}/releases?per_page=5`;
const response = await fetch(source, { headers: { Accept: 'application/vnd.github+json' } });
if (!response.ok) throw new Error(`GitHub releases: ${response.status}`);
interface Release { tag_name: string; name: string; html_url: string; published_at: string; draft: boolean; prerelease: boolean; body: string; assets: Array<{ name: string; size: number; browser_download_url: string; state: string }> }
const entries = await response.json() as Release[];
const prefix = `https://github.com/${repository}/releases/`;
const releases = await Promise.all(entries.filter((entry) => !entry.draft && entry.published_at && entry.html_url.startsWith(prefix)).map(async (entry) => {
  const prose = (entry.body ?? '').replace(/\r/g, '');
  const bullets: string[] = [];
  let continuing = false;
  for (const line of prose.split('\n')) {
    if (/^[-*] /.test(line)) { bullets.push(line.slice(2)); continuing = true; }
    else if (continuing && line.trim() && !/^\s*(#|```)/.test(line)) bullets[bullets.length - 1] += ` ${line.trim()}`;
    else continuing = false;
  }
  const highlights = bullets.slice(0, 3).map((line) => line.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1').replace(/https?:\/\/\S+/g, '').replace(/\s+by @\S+\s*(?:in)?\s*$/, '').replace(/[*`]/g, '').trim());
  // Some releases publish only a comparison link. Use its actual commit titles,
  // rather than guessing release features from the application version.
  const comparison = prose.match(/https:\/\/github\.com\/nglmercer\/TikTools-app\/compare\/([\w.~-]+\.\.\.[\w.~-]+)/)?.[1];
  if (!highlights.length && comparison) {
    const changes = await fetch(`https://api.github.com/repos/${repository}/compare/${comparison}`);
    if (!changes.ok) throw new Error(`GitHub comparison: ${changes.status}`);
    const data = await changes.json() as { commits: Array<{ commit: { message: string } }> };
    highlights.push(...data.commits.slice(-3).map((commit) => commit.commit.message.split('\n')[0]!));
  }
  return {
  tag: entry.tag_name, name: entry.name || entry.tag_name, url: entry.html_url,
  publishedAt: entry.published_at, prerelease: entry.prerelease,
  highlights,
  assets: entry.assets.filter((asset) => asset.state === 'uploaded' && asset.browser_download_url.startsWith(`${prefix}download/`)).map((asset) => ({ name: asset.name, size: asset.size, url: asset.browser_download_url })),
  };
}));
if (!releases.length) throw new Error('GitHub returned no published releases; retaining the existing snapshot.');
await writeFile(new URL('../docs/experience/releases.json', import.meta.url), JSON.stringify({ source, checkedAt: new Date().toISOString(), releases }, null, 2) + '\n');
console.log(`Updated documentation snapshot: ${releases.length} published releases.`);
