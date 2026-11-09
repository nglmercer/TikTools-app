# Settings and connections refinement

Follow-up to PR #62, iterated after the user requested the earlier OBS-like connection layout and fewer settings categories.

## Current organization

- **General:** language, application update preferences and a runtime-variables disclosure. The section no longer contains only a language selector.
- **Plugins:** plugin update channel/policy/interval and installation sources/trust, including the existing consent requirements.
- **Account & Cloud:** account/session, supported synchronization when signed in, and session diagnostics.
- **Collaboration:** status, devices/requests, network/security. Warnings, permission review, confirmation dialogs and drafts remain owned by the original feature.
- Legacy `updates`, `advanced` and `globals` links map to General, Plugins and expanded runtime variables respectively.

Connections restores the earlier two-column LIVE/explorer workspace with gray panel headers, full-width server panel and alternating server rows. Narrow screens stack the panels. Exploration still requires the explicit Explore action.

Visible checkboxes in the scoped settings features now use the existing native-backed BaseSwitch with role=switch and keyboard/disabled semantics. Connection schema forms opt into switches; the default schema control remains unchanged for automation editors. SettingsDisclosure replaces native details/summary in settings and collaboration, links button/region IDs, supports keyboard activation and retains mounted content and drafts. Permission review remains expanded initially.

## Scope protection

AutomationsPage.vue and its tests still match origin/remake. No automation feature file was edited in this iteration. Schema switches are opt-in only for PluginConnectionEditor and tested against the unchanged default. Native/backend contracts were not modified.

## Validation

- Full web suite: 152 files / 866 tests passed.
- After the final disclosure typing adjustment: 3 affected files / 13 tests passed.
- TypeScript, ESLint with zero warnings and frontend production build passed. Final spacing/width changes were rebuilt and checked in Chromium.
- Real headless host and Chromium at 1440x1000 and 390x844: parallel desktop panels/mobile stacking, switch activation with Space, disclosure activation with Enter/Space, no native details or checkbox roles in Settings, draft retention after collapsing and switching collaboration views, no page errors or document overflow.
- The unchanged native host was compiled from the preceding PR source using local-web. No mocked API responses were used for screenshots.

See [current captures](ui/settings-obs/README.md). The earlier flat-ui and settings-current galleries are historical.

## Limitations

The disposable host is signed out, has no installed plugins/server rows and does not support remote controllers. Those actual states appear in screenshots. No authenticated Cloud synchronization, installation, invitation creation, live connection, populated plugin credentials or native desktop accessibility certification is claimed. Server row styling and opt-in schema controls are covered by code/tests, not fabricated screenshot data.
