const preferredTones: Record<string, number> = {
  automationEvents: 0,
  automationActions: 1,
  automationVariables: 2,
  automationEventChat: 3,
  automationEventGift: 4,
  automationEventSocial: 5,
  automationEventRoom: 6,
  automationActionMedia: 7,
  automationActionTts: 1,
  automationActionHttp: 2,
  automationActionVars: 3,
  automationFilters: 4,
  automationCooldowns: 5,
  automationTemplates: 6,
};

export function tutorialToneClass(id: string): string {
  let tone = preferredTones[id];
  if (tone === undefined) {
    let hash = 2166136261;
    for (const char of id) {
      hash ^= char.charCodeAt(0);
      hash = Math.imul(hash, 16777619) >>> 0;
    }
    tone = hash % 8;
  }
  return `tutorial-tone-${tone}`;
}
