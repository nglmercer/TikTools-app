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
  feed: {
    view: { title: 'Choose the feed view', description: 'Live feed shows chat, gifts and follows as cards; switch to List for the compact log.' },
    filters: { title: 'Filter the activity', description: 'Narrow by kind or search by viewer and message. Filters also apply to the live feed cards.' },
    stage: { title: 'Read the live wall', description: 'Important messages stay on the wall. Likes and joins appear briefly as reactions, then fade out.' },
    controls: { title: 'Pause, clear and inspect', description: 'Pause auto-scroll to read at your own pace, clear the saved history, or open the inspector for details.' },
  },
  backup: {
    plan: { title: 'Know your limits', description: 'Each plan has its own storage and backup limits. These meters show how much of them you are using.' },
    tools: { title: 'Cloud or this PC', description: 'Switch between saving to your cloud account or working with files on this PC: export your data, or upload a file you already have.' },
    create: { title: 'Save a backup', description: 'Name a backup and save it to your account. It captures your rules, profiles, settings and cloud files, never secrets or cookies.' },
    list: { title: 'Download, restore or delete', description: 'Every saved backup can be downloaded, restored on any device signed in with this account, or deleted. Restoring always shows what will change first.' },
  },
    analytics: {
      range: { title: "Choose the date range", description: "Pick a preset or custom dates. The summary reloads for the active creator; changing the range never changes the stored events." },
      metrics: { title: "Pick the metric", description: "Cards compare chats, gifts, likes and joins. The selected metric drives the activity chart without mutating analytics data." },
      activity: { title: "Read the activity chart", description: "Use the chart to spot active days and hours. Empty periods can mean no LIVE session or no matching events." },
      facts: { title: "Check the facts", description: "Totals and session counts give a compact overview before comparing individual viewers." },
      contributors: { title: "Search contributors", description: "Sort or search the table to find the most active viewers. This is a read-only ranking." },
    },
    variables: {
      library: { title: "Open Variables", description: "Variables live with automations. Open the Variables tab to review values shared by events and actions." },
      values: { title: "Review values and secrets", description: "Local values can be marked secret. The editor never reveals a stored secret and saving it is always explicit." },
      review: { title: "Compare before synchronizing", description: "Use Changes to compare local and Cloud revisions. Review the difference first; raw secret values are not uploaded by the guide." },
    },
    settings: {
      sections: { title: "Choose a settings section", description: "General contains application preferences. Account contains the cloud profile, plan and backup tools." },
      language: { title: "Change the language", description: "The language applies immediately to the interface. It does not change LIVE data or automation behavior." },
      review: { title: "Choose the review mode", description: "Review mode controls how much confirmation sensitive synchronization flows ask for. Safer modes require more explicit review." },
      updater: { title: "Check updates manually", description: "Use the updater card to see the channel and check for updates. If updates are unavailable, download the release through the documented recovery path." },
    },
    overlayObs: {
      gateway: { title: "Start with the gateway", description: "OBS widgets receive events through the local gateway. Confirm it is running before copying any source URL." },
      widget: { title: "Choose the widget", description: "Pick the follow, gift, chat, likes or ranking source you want to place in OBS." },
      preview: { title: "Test with the preview", description: "Replay sample events and adjust the design locally. The preview does not prove that OBS is connected." },
      url: { title: "Copy the OBS URL", description: "Copy the source URL only after the gateway is ready, then paste it into an OBS Browser Source with the intended size." },
      design: { title: "Tune motion and appearance", description: "The design editor now includes animation style, duration, easing and stagger. Save before copying a refreshed URL." },
    },
    cloudOverlays: {
      open: { title: "Open the Cloud library", description: "When Cloud overlays are available, browse shared overlay documents from the Cloud library button." },
      widgets: { title: "Select a local target", description: "Choose which local widget should receive the selected Cloud design before applying anything." },
      preview: { title: "Preview before publishing", description: "Use the local preview to compare the selected design. Publishing or overwriting a Cloud copy remains an explicit action." },
      url: { title: "Copy the correct source", description: "Local gateway URLs and Cloud overlay URLs solve different recovery cases. Verify which one your OBS scene should use." },
    },
    automationAdvanced: {
      library: { title: "Open an event deliberately", description: "Advanced behavior lives inside an event. Select or create a disabled event before experimenting." },
      editor: { title: "Review filters, cooldown and order", description: "Inside the editor, conditions, alternatives, cooldown scope, random mode and delays decide exactly when actions run." },
      templates: { title: "Use templates as a starting point", description: "Templates can encode safer defaults. Importing still requires review and does not enable the rule by itself." },
      runs: { title: "Verify with tests and history", description: "Use test runs and the history panel to confirm timing, skipped actions and cooldown behavior before enabling the event." },
    },
    rankings: {
      tabs: { title: "Rankings or competitions", description: "Rankings collect scores. Competitions compare selected rankings during a bounded event." },
      list: { title: "Choose or create a ranking", description: "The list shows metric, mode and status. Create a new ranking only after deciding what viewer action should score." },
      detail: { title: "Inspect the live scoreboard", description: "A detail view shows score, pause state and integrity. Pause, resume and reset are explicit operations." },
      competitions: { title: "Configure competitions", description: "Competitions need compatible rankings and clear boundaries. They are experimental, so verify the overlay before using them on air." },
    },
    pluginSetup: {
      catalog: { title: "Open the installed plugin", description: "Start from Installed, not Explore, when configuring a plugin already trusted on this machine." },
      details: { title: "Review permissions and state", description: "The details header shows enabled state, version and actions that affect the runtime. Review permissions before enabling." },
      settings: { title: "Configure settings", description: "The Settings tab contains plugin-specific values. Save first when the plugin asks before testing." },
      connection: { title: "Connect and test", description: "The Connection tab manages external service credentials and offers a test action when the plugin provides one." },
    },
    connectionRecovery: {
      form: { title: "Recheck the creator and state", description: "Start with the exact creator handle and the current connection state. Offline, authentication and unavailable-room states need different recovery." },
      explorer: { title: "Verify the LIVE room", description: "Refresh the room list and confirm the room is accessible. A missing room can be offline, restricted or simply not available to guests." },
      plugins: { title: "Check integrations after reconnecting", description: "Connection integrations can lag behind a reconnect. Review their health before assuming automations or overlays failed." },
    },
    feedInspector: {
      controls: { title: "Open the inspector", description: "Use the inspector control in the feed toolbar. It does not pause the LIVE or change stored events." },
      panel: { title: "Understand the panel", description: "The inspector has viewers, event details and profile tabs. Its content follows the event or viewer you select." },
      viewers: { title: "Compare active viewers", description: "Switch between top viewers and likes to see who is driving activity in the current session." },
      details: { title: "Inspect one event", description: "Event details include counts, points, time and message evidence useful when diagnosing automation behavior." },
      profile: { title: "Open a viewer profile", description: "The profile tab summarizes the viewer and their recent messages without exposing private account data." },
    },
    backupRestore: {
      tools: { title: "Choose the restore source", description: "Cloud backups and local files are separate recovery paths. Pick the source before reviewing what will change." },
      list: { title: "Restore deliberately", description: "A backup row can download, restore or delete. Restore always shows a review of affected data before applying it." },
      plan: { title: "Verify the result", description: "After restoration, check the plan, profiles, settings and key automations. A successful restore does not revalidate every external credential." },
    },
} as const;
