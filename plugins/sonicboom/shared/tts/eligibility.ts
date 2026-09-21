import type {
  SpecialTtsUser,
  TtsAuthor,
  TtsCommentDecision,
  TtsDecision,
  TtsEligibility,
  TtsRequest,
  TtsSettings,
} from './types.ts';
import { TTS_LIMITS, normalizeHandle } from './settings.ts';
import { chooseVoice } from './voice.ts';

/** Applies the comment-mode filter and strips the command prefix when asked. */
export function evaluateCommentFilter(comment: string, settings: TtsSettings): TtsCommentDecision {
  const text = comment.slice(0, TTS_LIMITS.maxCommentLength);
  const trimmed = text.trim();
  if (!trimmed) {
    return { allowed: false, spokenText: '', reason: 'Empty comments are never spoken.' };
  }
  switch (settings.commentMode) {
    case 'any':
      return { allowed: true, spokenText: trimmed, reason: 'Any comment may be spoken.' };
    case 'dot': {
      if (!trimmed.startsWith('.') || trimmed.length < 2) {
        return { allowed: false, spokenText: '', reason: 'Only comments starting with `.` are spoken.' };
      }
      const spoken = settings.stripCommand ? trimmed.slice(1).trim() : trimmed;
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping `.`.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Dot-prefixed comment.' };
    }
    case 'slash': {
      if (!trimmed.startsWith('/') || trimmed.length < 2) {
        return { allowed: false, spokenText: '', reason: 'Only comments starting with `/` are spoken.' };
      }
      const spoken = settings.stripCommand ? trimmed.slice(1).trim() : trimmed;
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping `/`.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Slash-prefixed comment.' };
    }
    case 'command': {
      const command = settings.command.trim();
      if (!command) {
        return { allowed: false, spokenText: '', reason: 'No TTS command is configured.' };
      }
      const lowered = trimmed.toLowerCase();
      const prefix = command.toLowerCase();
      if (lowered !== prefix && !lowered.startsWith(`${prefix} `)) {
        return { allowed: false, spokenText: '', reason: `Only comments starting with \`${command}\` are spoken.` };
      }
      if (!settings.stripCommand) {
        return { allowed: true, spokenText: trimmed, reason: 'Command-prefixed comment.' };
      }
      const spoken = trimmed.slice(command.length).trim();
      if (!spoken) {
        return { allowed: false, spokenText: '', reason: 'Nothing remains after stripping the command.' };
      }
      return { allowed: true, spokenText: spoken, reason: 'Command-prefixed comment.' };
    }
  }
}

export function findSpecialUser(handle: string, settings: TtsSettings): SpecialTtsUser | undefined {
  const clean = normalizeHandle(handle);
  if (!clean) return undefined;
  return settings.specialUsers.find((entry) => entry.handle === clean);
}

/** Special-user allow/block plus the global allow rules, in priority order. */
export function evaluateUserEligibility(author: TtsAuthor, settings: TtsSettings): TtsEligibility {
  const special = findSpecialUser(author.handle, settings);
  if (special) {
    if (!special.allowed) {
      return { allowed: false, reason: 'This user is blocked in Special Users.', via: 'special-user', specialUser: special };
    }
    return { allowed: true, reason: 'Allowed by Special Users.', via: 'special-user', specialUser: special };
  }
  if (settings.allowAllUsers) {
    return { allowed: true, reason: 'All users may use TTS.', via: 'all-users' };
  }
  const clean = normalizeHandle(author.handle);
  if (settings.allowListedUsers && clean && settings.allowedUsers.includes(clean)) {
    return { allowed: true, reason: 'Listed in Allowed Users.', via: 'allow-list' };
  }
  const roles = author.roles ?? {};
  if (settings.allowSubscribers && roles.isSubscriber === true) {
    return { allowed: true, reason: 'Subscribers may use TTS.', via: 'subscriber' };
  }
  // The remaining roles grant access only with authoritative data. A missing
  // flag means "unknown", which never satisfies the rule.
  if (settings.allowFollowers && roles.isFollower === true) {
    return { allowed: true, reason: 'Followers may use TTS.', via: 'follower' };
  }
  if (settings.allowModerators && roles.isModerator === true) {
    return { allowed: true, reason: 'Moderators may use TTS.', via: 'moderator' };
  }
  if (settings.allowTeamMembers && roles.isTeamMember === true) {
    const level = typeof roles.teamLevel === 'number' ? roles.teamLevel : 0;
    if (level >= settings.minTeamLevel) {
      return { allowed: true, reason: 'Team members may use TTS.', via: 'team-member' };
    }
    return { allowed: false, reason: `Team level ${settings.minTeamLevel} or higher is required.`, via: 'none' };
  }
  if (settings.allowTopGifters && roles.isTopGifter === true) {
    const rank = typeof roles.topGifterRank === 'number' ? roles.topGifterRank : Number.MAX_SAFE_INTEGER;
    if (rank <= settings.topGifterCount) {
      return { allowed: true, reason: 'Top gifters may use TTS.', via: 'top-gifter' };
    }
    return { allowed: false, reason: `Only the top ${settings.topGifterCount} gifters may use TTS.`, via: 'none' };
  }
  return { allowed: false, reason: 'This user is not allowed to use TTS.', via: 'none' };
}

/** `true` when the viewer can cover the per-message points cost. */
export function canAffordTts(points: number | undefined, settings: TtsSettings): boolean {
  if (!settings.chargePoints || settings.pointsCost <= 0) return true;
  return typeof points === 'number' && Number.isFinite(points) && points >= settings.pointsCost;
}

/** Single full-pipeline decision. Callers must check `speak` before acting:
 * computing eligibility never speaks by itself (old `shouldRead` bug fix). */
export function decideTts(request: TtsRequest): TtsDecision {
  const { comment, author, settings } = request;
  const idle = (reason: string): TtsDecision => ({
    speak: false,
    reason,
    spokenText: '',
    voice: '',
    language: settings.language,
    pointsCost: 0,
    via: 'none',
  });
  if (!settings.enabled) {
    return idle('TTS is disabled.');
  }
  const commentDecision = evaluateCommentFilter(comment, settings);
  if (!commentDecision.allowed) {
    return idle(commentDecision.reason);
  }
  const eligibility = evaluateUserEligibility(author, settings);
  if (!eligibility.allowed) {
    return {
      speak: false,
      reason: eligibility.reason,
      spokenText: '',
      voice: '',
      language: settings.language,
      pointsCost: 0,
      via: eligibility.via,
      specialUser: eligibility.specialUser,
    };
  }
  const pointsCost = settings.chargePoints ? Math.max(0, Math.round(settings.pointsCost)) : 0;
  if (pointsCost > 0 && !canAffordTts(author.points, settings)) {
    return {
      speak: false,
      reason: `Needs ${pointsCost} points to use TTS.`,
      spokenText: '',
      voice: '',
      language: settings.language,
      pointsCost,
      via: eligibility.via,
      specialUser: eligibility.specialUser,
    };
  }
  const voice = chooseVoice(
    settings,
    eligibility.specialUser?.voice,
    request.availableVoices,
    request.randomFn,
  );
  return {
    speak: true,
    reason: eligibility.reason,
    spokenText: commentDecision.spokenText,
    voice,
    language: settings.language,
    pointsCost,
    via: eligibility.via,
    specialUser: eligibility.specialUser,
  };
}
