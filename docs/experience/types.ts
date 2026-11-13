export type Topic = 'connection' | 'automation' | 'overlays' | 'points';
export interface ExperienceCopy {
  interactive: string; fictional: string; tryIt: string; reset: string; openFull: string; close: string;
  preview: string; loading: string; task: string; success: string;
  topicsTitle: string; topicsDescription: string; readGuide: string;
  topics: Record<Topic, { title: string; description: string; task: string; result: string }>;
  otherTasks: Record<'feed' | 'analytics' | 'plugins' | 'cloud' | 'collaboration' | 'settings' | 'diagnostics', { task: string; result: string }>;
  previewLabels: { creator: string; event: string; action: string; output: string; chat: string; gift: string; connected: string; disconnected: string; message: string };
  connectionStates: { label: string; normal: string; offline: string; auth: string; offlineHelp: string; authHelp: string };
  releases: { stable: string; prerelease: string; notes: string; source: string; downloads: string; noDownloads: string; checksums: string; published: string; snapshot: string; refresh: string; all: string; highlights: string; unavailable: string };
}
