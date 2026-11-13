/** Self-contained because VitePress serializes this callback for browser search. */
export function tokenizeDocSearch(text: string): string[] {
  const normalized = text.normalize('NFKC');
  const tokens = new Set(Array.from(new Intl.Segmenter('ja', { granularity: 'word' }).segment(normalized))
    .filter((part) => part.isWordLike).map((part) => part.segment));
  for (const token of [...tokens]) for (const part of token.split(/[^\p{L}\p{N}]+/u)) if (part) tokens.add(part);
  // Adjacent CJK pairs keep compound searches useful across dictionary boundaries
  // without shipping another tokenizer or a large browser dictionary.
  for (const match of normalized.matchAll(/[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}]+/gu)) {
    const characters = Array.from(match[0]);
    for (let index = 0; index < characters.length - 1; index++) tokens.add(characters[index]! + characters[index + 1]!);
  }
  return [...tokens];
}
