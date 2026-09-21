export type TtsCommentMode = 'any' | 'dot' | 'slash' | 'command';

export type SpecialTtsUser = {
  handle: string;
  allowed: boolean;
  voice: string;
  speed: number;
  pitch: number;
};

export type TtsSettings = {
  enabled: boolean;
  language: string;
  defaultVoice: string;
  randomVoice: boolean;
  defaultSpeed: number;
  defaultPitch: number;
  volume: number;

  allowAllUsers: boolean;
  allowFollowers: boolean;
  allowSubscribers: boolean;
  allowModerators: boolean;
  allowTeamMembers: boolean;
  minTeamLevel: number;
  allowTopGifters: boolean;
  topGifterCount: number;
  allowListedUsers: boolean;
  allowedUsers: string[];

  commentMode: TtsCommentMode;
  command: string;
  stripCommand: boolean;

  chargePoints: boolean;
  pointsCost: number;

  specialUsers: SpecialTtsUser[];
};

/** Authoritative role flags for one chat author. Absent means unknown. */
export type TtsAuthorRoles = {
  isSubscriber?: boolean;
  isFollower?: boolean;
  isModerator?: boolean;
  isTeamMember?: boolean;
  teamLevel?: number;
  isTopGifter?: boolean;
  topGifterRank?: number;
};

export type TtsAuthor = {
  handle: string;
  points?: number;
  roles?: TtsAuthorRoles;
};

export type TtsCommentDecision = {
  allowed: boolean;
  spokenText: string;
  reason: string;
};

export type TtsEligibility = {
  allowed: boolean;
  reason: string;
  via: 'special-user' | 'all-users' | 'allow-list' | 'subscriber' | 'follower' | 'moderator' | 'team-member' | 'top-gifter' | 'none';
  specialUser?: SpecialTtsUser;
};

export type TtsDecision = {
  speak: boolean;
  reason: string;
  spokenText: string;
  voice: string;
  language: string;
  pointsCost: number;
  via: TtsEligibility['via'];
  specialUser?: SpecialTtsUser;
};

export type TtsRequest = {
  comment: string;
  author: TtsAuthor;
  settings: TtsSettings;
  availableVoices: readonly string[];
  randomFn?: () => number;
};

export type TtsDeduperOptions = {
  windowMs?: number;
  maxEntries?: number;
};

export type TtsLogEntry = {
  id: number;
  at: number;
  ok: boolean;
  source: 'tester' | 'auto';
  text: string;
  voice: string;
  summary: string;
};
