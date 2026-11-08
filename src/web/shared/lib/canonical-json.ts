/** New Owner library wire canonical form. Keys use Unicode scalar order;
 * numbers use ECMAScript shortest form, also used by native ryu-js. */
export function scalarCompare(left: string, right: string): number {
  const a = Array.from(left, (character) => character.codePointAt(0)!); const b = Array.from(right, (character) => character.codePointAt(0)!);
  for (let i = 0; i < Math.min(a.length, b.length); ++i) if (a[i] !== b[i]) return a[i]! - b[i]!;
  return a.length - b.length;
}
export function canonicalJson(value: unknown): string {
  const chunks: string[] = []; let bytes = 0; let nodes = 0;
  const encoder = new TextEncoder();
  function append(chunk: string): void {
    bytes += encoder.encode(chunk).length;
    if (bytes > 8 * 1024 * 1024) throw new Error('Canonical JSON exceeds its byte budget.');
    chunks.push(chunk);
  }
  function write(node: unknown, depth: number): void {
    if (depth > 128 || ++nodes > 4_194_304) throw new Error('Canonical JSON exceeds its structural budget.');
    if (node === null || typeof node === 'boolean' || typeof node === 'string') { append(JSON.stringify(node)); return; }
    if (typeof node === 'number') {
      if (!Number.isFinite(node) || Number.isInteger(node) && !Number.isSafeInteger(node)) throw new Error('Source number cannot be represented exactly.');
      append(JSON.stringify(node)); return;
    }
    if (Array.isArray(node)) {
      append('['); node.forEach((entry, index) => { if (index) append(','); write(entry, depth + 1); }); append(']'); return;
    }
    if (typeof node !== 'object' || node === undefined) throw new Error('Canonical JSON requires JSON values.');
    append('{'); Object.keys(node).sort(scalarCompare).forEach((key, index) => {
      if (index) append(','); append(JSON.stringify(key)); append(':'); write((node as Record<string, unknown>)[key], depth + 1);
    }); append('}');
  }
  write(value, 0); return chunks.join('');
}
