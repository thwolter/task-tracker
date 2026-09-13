# Tempo release roadmap

This list is ordered by release dependency: complete the App Store blockers
first, then protect user data and polish the first-run experience. Later ideas
should not expand the local-first scope until the first release is stable.

## P0 — Mac App Store submission blockers

### Distribution and security

- [ ] Create a Mac App Store release configuration with the App Sandbox
  entitlement enabled.
- [ ] Add the least-privilege sandbox entitlements required by the app:
  user-selected file read/write access for Markdown export and an app-container
  location for local data.
- [ ] Move the SQLite database to the sandbox-compatible Application Support
  container and test a clean install, upgrade, and relaunch.
- [ ] Add a signed distribution build pipeline: Mac App Distribution
  certificate, provisioning profile, hardened release validation, and an
  uploadable App Store artifact.
- [ ] Verify the final artifact on a clean macOS account, including export and
  database persistence.

### Privacy, support, and store metadata

- [ ] Replace every placeholder in `website/datenschutz.html` with the real
  controller, contact address, email, and hosting provider.
- [ ] Add a privacy-policy link inside the app, alongside a support/contact
  link.
- [ ] Publish final privacy and support URLs, then enter accurate App Privacy
  details in App Store Connect. The app is intended to declare no collected
  data while it remains local-only.
- [ ] Prepare final App Store metadata: product name, subtitle, description,
  keywords, category, age rating, copyright, and localized German copy.
- [ ] Capture polished Mac screenshots for the product page and write concise
  reviewer notes explaining the fully offline workflow.

### Release quality gate

- [ ] Fix the strict Clippy warning in `src/language.rs` and make Clippy a
  required CI check.
- [ ] Add a release-build CI job for macOS that tests the packaged app, not
  only the development binary.
- [ ] Establish a manual preflight checklist for clean install, relaunch,
  offline operation, export failure handling, localization, and upgrade
  compatibility.

## P1 — First-release user trust and quality

### Accessibility and Mac interaction

- [ ] Give every custom control an accessible role and concise label,
  especially icon-only Evaluation, Settings, Edit, Delete, and archive
  controls.
- [ ] Support keyboard navigation and activation throughout: Tab/Shift-Tab,
  Enter/Space, Escape to dismiss dialogs, and visible focus states.
- [ ] Test common workflows using VoiceOver; ensure page changes, dialogs, and
  transient status messages are announced without moving the reading position
  unexpectedly.
- [ ] Add a native-feeling application menu and shortcuts for New Project,
  Start/Pause/End Timer, Export, Settings, and Help.
- [ ] Persist the window size and test common Mac window sizes, including
  narrow and large layouts.

### Safe data management

- [ ] Confirm destructive actions before deleting a session or permanently
  deleting a project and its sessions.
- [ ] Provide a short Undo action after deletion where practical.
- [x] Add a complete raw-data export format (CSV and/or JSON), not only the
  formatted Markdown report.
- [ ] Add backup, restore, and a clearly explained “Delete all local data”
  action.
- [ ] Display an About screen with the app version, privacy policy, support
  contact, and third-party license notices.

### Timer resilience and first use

- [ ] On relaunch after an interrupted timer, ask whether to resume, finish at
  the last saved checkpoint, or discard the session instead of silently
  resolving it.
- [ ] Add a first-run experience that helps a user create or rename their
  first project and start a session.
- [ ] Add useful empty states for Evaluation and project drill-down views.
- [ ] Explain the timer’s pause, restart, and recovery behavior in a compact
  in-app help surface.

## P2 — Product capability expansion after launch

### Reporting and workflows

- [ ] Add custom date ranges and filters by project.
- [ ] Add optional tags, billable status, and hourly-rate reporting.
- [ ] Add project goals or time budgets with progress indicators.
- [ ] Add richer report formats only where they improve on CSV/Markdown, such
  as a shareable PDF summary.

### Focus conveniences

- [ ] Add an optional menu-bar timer with Start/Pause/End controls.
- [ ] Add optional reminders to start, pause, or finish a session.
- [ ] Add a global shortcut for bringing Tempo forward or toggling the timer,
  with a user-controlled setting.

### Optional connected features

- [ ] Evaluate encrypted, opt-in sync only after defining the account,
  conflict-resolution, privacy, backup, and deletion model.
- [ ] Consider calendar or task-manager integrations only as explicit,
  permission-based imports; retain a fully useful offline experience without
  them.

## Suggested release sequence

1. Complete P0 and submit a private TestFlight build.
2. Address P1 accessibility, data safety, and timer recovery feedback from
   testing.
3. Submit the first Mac App Store release.
4. Choose P2 work from customer feedback rather than committing to sync or
   integrations in advance.
