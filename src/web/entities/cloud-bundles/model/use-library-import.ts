import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import { importLibraryDocument, type LibraryImportEnvironment, type LibraryImportSource } from '../lib/library-import.ts';

/**
 * Imports Cloud templates and profiles for the current account and connection. Every
 * entry point (bundles, Settings, the Cloud library) goes through this one coordinator,
 * and the review it opens is the single owner-shell dialog.
 */
export function useLibraryImport(): {
  importTemplate(source: LibraryImportSource): Promise<void>;
  importProfile(source: LibraryImportSource): Promise<void>;
} {
  const client = useControlClient();
  const account = useAccountStore();
  const cloud = useCloudContext();
  const environment: LibraryImportEnvironment = {
    client,
    identity: () => {
      const status = account.status;
      return status?.state === 'signed-in' && status.origin && status.accountId ? { origin: status.origin, accountId: status.accountId } : null;
    },
    stamp: cloud.stamp,
    isCurrent: cloud.isCurrent,
  };
  return {
    importTemplate: (source) => importLibraryDocument(environment, 'template', source),
    importProfile: (source) => importLibraryDocument(environment, 'profile', source),
  };
}
