import type { ExperienceCopy } from '../types.ts';

const copy: ExperienceCopy = {
  interactive: 'Interactive example', fictional: 'Fictional data', tryIt: 'Try the controls', reset: 'Reset example', openFull: 'Open full demo', close: 'Close example',
  preview: 'Preview', loading: 'Loading the real interface…', task: 'Your task', success: 'What to look for',
  topicsTitle: 'Learn by doing', topicsDescription: 'A real interface, a small task, and a result you can check. Pick your next workflow.', readGuide: 'Read the guide',
  topics: {
    connection: { title: 'Connect your LIVE', description: 'Choose a creator and see audience events arrive.', task: 'Connect demo_creator, then inspect an incoming event.', result: 'The connection becomes active and fictional events appear in the feed.' },
    automation: { title: 'Turn events into actions', description: 'Build a chat rule and inspect its execution.', task: 'Explore the sample action, test it, and inspect the run result.', result: 'The automation history shows a completed run with its rendered result.' },
    overlays: { title: 'Design for your broadcast', description: 'Preview a widget and prepare its OBS browser source.', task: 'Choose a widget, adjust its appearance, and replay the preview.', result: 'The preview reflects your design. Real OBS delivery needs a running gateway.' },
    points: { title: 'Reward your audience', description: 'Set earning rules and understand viewer balances.', task: 'Inspect the earning rules and compare a viewer’s balance after an event.', result: 'The sample balance updates. The demo uses fixed example awards; real rates need a LIVE check.' },
  },
  otherTasks: {
    feed: { task: 'Advance sample events, select a feed entry, and open the viewer profile.', result: 'Fictional event details and the viewer’s tracked activity appear without connecting to TikTok.' },
    analytics: { task: 'Read the current totals, advance a sample event, and compare the updated activity.', result: 'The totals reflect the simulated event. They do not measure a real broadcast.' },
    plugins: { task: 'Inspect the sample plugin, review its permissions, and try its simulated installation controls.', result: 'The sample plugin state changes inside the demo; no package, service or audio device is contacted.' },
    cloud: { task: 'Compare the local greeting with the Cloud example and choose which reviewed change to apply.', result: 'The fictional comparison is resolved locally. No Cloud account or remote data changes.' },
    collaboration: { task: 'Review the fictional device request and approve only the limited access you intend to grant.', result: 'The sample identity appears with the reviewed permissions. No external device joins this demo.' },
    settings: { task: 'Change a local preference, inspect its effect, then reset the demo.', result: 'The preference applies to this demo instance; reset restores its initial state.' },
    diagnostics: { task: 'Select the failed entry, inspect its details, and retry a simulated action.', result: 'You can trace the sample failure and its retry outcome without contacting an external service.' },
  },
  previewLabels: { creator: 'Creator', event: 'Audience event', action: 'Automation', output: 'Broadcast', chat: 'Chat message', gift: 'Gift received', connected: 'Connected', disconnected: 'Disconnected', message: 'Hello TikTools!' },
  connectionStates: { label: 'Connection example', normal: 'Successful connection', offline: 'Creator offline', auth: 'Session required', offlineHelp: 'The creator is not live. Confirm their broadcast has started before trying again.', authHelp: 'This room requires an authorized TikTok session. In the desktop app, use a fresh session cookie in the masked field. Never paste credentials into this demo.' },
  releases: { stable: 'Stable', prerelease: 'Prerelease', notes: 'Release notes', source: 'Source code', downloads: 'Downloads', noDownloads: 'No downloads for this system in this snapshot.', checksums: 'SHA-256 checksums', published: 'Published', snapshot: 'Release snapshot verified', refresh: 'Check the latest on GitHub', all: 'All systems', highlights: 'Highlights', unavailable: 'Release data is unavailable. View the published releases on GitHub.' },
};
export default copy;
