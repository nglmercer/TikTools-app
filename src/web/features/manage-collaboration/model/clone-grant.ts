import type { Grant } from '@/entities/collaboration/api/types.ts';

/** A draft edit never mutates an approved or externally supplied grant in place. */
export function cloneGrant(grant: Grant): Grant {
  return JSON.parse(JSON.stringify(grant)) as Grant;
}
