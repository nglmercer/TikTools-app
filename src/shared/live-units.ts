/** Display labels only; values and unit identifiers are never converted. */
export function liveUnitLabel(unit: string): string {
  return ({ twitch_bits: 'Bits', tiktok_diamonds: 'Diamonds' } as Record<string, string>)[unit] ?? unit;
}
