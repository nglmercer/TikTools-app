import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import type { InstallError, InstallErrorCode, PackagePreview } from '../api/package-types.ts';

const CODES: readonly InstallErrorCode[] = [
  'already-installed',
  'not-installed',
  'not-found',
  'no-compatible-release',
  'download-failed',
  'digest-mismatch',
  'signature-required',
  'signature-invalid',
  'untrusted-publisher',
  'revoked',
  'invalid-package',
  'incompatible',
  'consent-required',
  'downgrade-blocked',
  'busy',
  'cancelled',
  'io-error',
  'unsupported',
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export function isInstallErrorCode(value: unknown): value is InstallErrorCode {
  return typeof value === 'string' && (CODES as readonly string[]).includes(value);
}

export function asInstallError(value: unknown): InstallError | null {
  if (!isRecord(value) || !isInstallErrorCode(value.code) || typeof value.message !== 'string') return null;
  return { code: value.code, message: value.message, details: value.details };
}

export function installErrorOf(failure: unknown): InstallError | null {
  if (failure instanceof ControlCallError) return asInstallError(failure.data);
  return null;
}

export function installFailure(failure: unknown): InstallError {
  return installErrorOf(failure) ?? { code: 'unsupported', message: errorMessage(failure) };
}

export function reviewFromError(error: InstallError): PackagePreview | null {
  if (error.code !== 'consent-required' || !isRecord(error.details)) return null;
  return typeof error.details.consentToken === 'string' ? (error.details as unknown as PackagePreview) : null;
}

export function installErrorKey(code: InstallErrorCode): string {
  return `packages.errors.${code}`;
}
