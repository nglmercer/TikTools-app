export default {
  controls: { next: 'Next', previous: 'Previous', done: 'Finish guide', close: 'Close guide', progress: '{{current}} of {{total}}' },
  connection: {
    form: { title: 'Choose a creator', description: 'Enter the creator handle and review connection status. The guide never connects or reads your session cookie.' },
    explorer: { title: 'Explore LIVE rooms', description: 'Refresh or choose a room yourself. A successful demo connection is simulated; native access depends on the host and TikTok.' },
    plugins: { title: 'Check integrations', description: 'Review each installed connection and its health before enabling effects. Opening this guide does not contact a plugin service.' },
  },
  automation: {
    profiles: { title: 'Choose the working profile', description: 'Keep a profile for the rules you want to review. Switching or importing a profile is an explicit application action.' },
    library: { title: 'Review events and actions', description: 'An event links its trigger, filters and cooldown to reusable actions. Keep new effects disabled until their definition is reviewed.' },
    templates: { title: 'Start from a template', description: 'Open Rule templates to preview a reusable configuration. Importing and enabling require their own review; this guide performs neither.' },
  },
  plugin: {
    catalog: { title: 'Installed and Explore', description: 'Review installed packages or browse compatible catalog entries. Native and process plugins retain the operating-system permissions of their user.' },
    search: { title: 'Find a package', description: 'Search and inspect publisher, version and requirements. An empty catalog and an unavailable backend are different states.' },
    install: { title: 'Review installation', description: 'Installation has its own signature and permission review. The guide does not install, approve new permissions or run a test action.' },
  },
  overlay: {
    gateway: { title: 'Check gateway status', description: 'Live output requires the event-gateway plugin and a widget credential. A local preview does not prove OBS connectivity.' },
    preview: { title: 'Preview locally', description: 'The preview uses the same widget renderer with sample events. It needs no real LIVE session and sends no alerts to OBS.' },
    design: { title: 'Review the design', description: 'Edit and save intentionally, then copy an updated source URL for OBS. Overlay Changes remains an unmerged follow-up.' },
  },
  points: {
    rules: { title: 'Review earning rules', description: 'Inspect the configured values before changing them. Some controls are locked during a connected or busy LIVE session.' },
    leaderboard: { title: 'Inspect committed points', description: 'Search or sort viewers. A display refresh is not a points adjustment, reset or replay of LIVE events.' },
  },
  changes: {
    library: { title: 'Choose the records', description: 'Events, actions and variables have separate review boundaries. Unsaved editor candidates must be preserved before reviewing another state.' },
    review: { title: 'Review Changes', description: 'Automations Changes is local-only. Reviewed Variables uses native revision checks; raw variable values are not uploaded by this review.' },
    profiles: { title: 'Keep recovery explicit', description: 'An unknown acknowledgement must be resolved through its original journal. Dismissing this guide neither completes a task nor retries a mutation.' },
  },
  diagnostics: {
    log: { title: 'Inspect bounded history', description: 'Filter by source, level or text and inspect repeated entries. Review redacted exports before sharing; the history is session-only.' },
    back: { title: 'Review evidence and recover', description: 'Copy or export reviewed evidence, then use the original flow for conflicts, expired sessions or uncertain commands. Clearing logs does not repair the error.' },
  },
} as const;
