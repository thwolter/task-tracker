# Tempo store listing and launch brief

**Status:** working draft based on the implemented code as at 17 September 2026. It is not a submission-ready compliance declaration. Complete the release blockers before entering the fields in App Store Connect or Partner Center.

## Product truth

Tempo is a proprietary, local-first desktop app for focused work sessions. A person creates projects, tracks a session, adds an activity note, reviews recorded time by project and period, and exports the result.

| May be claimed | Must not be claimed |
| --- | --- |
| Local SQLite storage; no account or network service for task data | Automatic time/activity tracking or surveillance monitoring |
| Project creation, renaming, archiving/restoring, and confirmed permanent deletion | Invoicing, billable hours, rates, compliance reporting, or payroll support |
| Start, pause, resume, end, note, and manual start/finish-time correction | Cloud backup, synchronisation, collaboration, or a web/mobile app |
| Day/week/month/year evaluation; CSV, Markdown, and JSON export | A manual language or theme setting |
| System light/dark appearance; German for `de*` system locales and English otherwise | “Free”, “open source”, or source-code availability |

macOS additionally has a native application menu with backup/restore and keyboard shortcuts. Do not promote those as a Windows feature until an equivalent Windows UI is exposed and tested.

## Positioning and commercial recommendation

**Positioning:** “A calm, local desktop companion for choosing one piece of work, recording the session, and remembering what was done.” The close-out note and focused-session workflow are the differentiated promise; privacy supports it but is not the whole proposition.

**Recommended launch model:** a one-time purchase of **€4.99**, with no subscription, trial, ads, or in-app purchases. This is a recommendation, not a configured price. It is consistent with a self-contained utility and avoids promising recurring value before sync, teams, or ongoing services exist.

If a launch discount is wanted, use **€3.99** only as a time-limited introductory price and return to €4.99. Keep the same economic intent on the Microsoft Store, subject to each store’s local price tier/conversion choices.

## Proposed customer-facing metadata

The product text is deliberately limited to implemented features. It can be used as a starting point for the German (`de-DE`) and English (`en-US`) listings.

### Apple App Store — macOS

| Field | Proposed value |
| --- | --- |
| Name | Tempo |
| Subtitle (German) | Fokuszeit, lokal gespeichert |
| Subtitle (English) | Focus sessions, kept local |
| Primary category | Productivity |
| Price | €4.99, one-time purchase — commercial decision pending |
| Age rating | Complete Apple’s current questionnaire from the final build; this utility has no user-generated/public content, ads, gambling, or mature content features. Do not enter a rating from this statement alone. |
| Copyright | `2026 Thomas Wolter` — confirm the legal rights holder before submission. |
| SKU | `tempo-macos` — proposed internal identifier; immutable after the app record is created. |
| Bundle ID | `io.github.thwolter.task-tracker` is the current bundle metadata; reserve and use the final App ID explicitly before building. |
| Privacy policy | Final public URL required; the current website template still contains placeholders. |
| Support URL | Final public URL required and must lead to working contact information. |
| Marketing URL | Final Tempo landing-page URL, once deployed. |

**German promotional text** (≤170 characters):

> Fokuszeit erfassen, kurz festhalten und lokal auswerten — mit Projekten, Pausen, Exporten und ohne Konto.

**German description:**

> Tempo ist ein ruhiger Desktop-Begleiter für konzentrierte Arbeit. Lege Projekte an, starte eine Session und halte nach dem Ende kurz fest, was du erledigt hast.
>
> Deine Arbeitszeit bleibt übersichtlich: Pausiere und setze Sessions fort, korrigiere Start- und Endzeit bei Bedarf und prüfe deine Zeit nach Tag, Woche, Monat oder Jahr.
>
> Projekte lassen sich umbenennen, archivieren und wiederherstellen. Vergangene Einträge kannst du prüfen, Notizen bearbeiten oder einzelne Einträge löschen. Für die weitere Verwendung exportiert Tempo Auswertungen als CSV, Markdown oder JSON.
>
> Tempo speichert Projekte, Sessions und Notizen lokal in einer SQLite-Datenbank. Es braucht kein Konto und überträgt keine Arbeitsdaten an einen externen Dienst.
>
> Tempo folgt dem hellen oder dunklen Erscheinungsbild deines Systems. Bei deutscher Systemsprache erscheint die Oberfläche auf Deutsch, sonst auf Englisch.

**English promotional text** (≤170 characters):

> Record focused work, add a short activity note, and review it locally — with projects, pause/resume, exports, and no account.

**English description:**

> Tempo is a calm desktop companion for focused work. Create projects, start a session, and capture a short note about what you completed.
>
> Keep time understandable: pause and resume sessions, correct a session’s start and finish time when needed, and review your time by day, week, month, or year.
>
> Rename, archive, and restore projects. Review past entries, edit notes, or delete individual completed entries. Export evaluations as CSV, Markdown, or JSON when you need the data elsewhere.
>
> Tempo stores projects, sessions, and notes locally in a SQLite database. It needs no account and sends no work data to an external service.
>
> Tempo follows your system light or dark appearance. It uses German for German system locales and English otherwise.

**Keywords** (keep below Apple’s 100-byte field limit; do not add competitors’ names):

- German: `zeiterfassung,fokus,timer,arbeitszeit,projekte,offline,notizen`
- English: `time tracker,focus timer,work sessions,offline,projects,notes`

**Reviewer note:**

> Tempo works fully offline and does not require an account. Create a project in the first-run screen, select it on Home, start a timer, then end it to reach the optional activity-note screen. Evaluation and Export are available from Home. The current macOS menu provides backup and restore.

### Microsoft Store — Windows

| Field | Proposed value |
| --- | --- |
| Product name | Tempo — reserve the name before packaging; the current MSIX display name is `Tempo Task Tracker`. Align the final reserved name, manifest, and listing. |
| Category | Productivity |
| Base price | €4.99-equivalent, one-time purchase — commercial decision pending |
| Description | Use the English description above for `en-us`; create a German listing from the German description above. |
| Product features | `Project-based focus sessions`; `Pause, resume, and activity notes`; `Day, week, month, and year evaluation`; `CSV, Markdown, and JSON export`; `Local SQLite storage with no account`; `System light/dark appearance`; `German and English interface` |
| Keywords | `time tracker,focus timer,work sessions,offline,projects,notes` (optional) |
| Copyright | `2026 Thomas Wolter` — confirm legal rights holder. |
| Privacy policy / support | Publish working URLs before release even where a particular Partner Center field is conditional; they are necessary for user trust and any actual data-handling declaration. |
| Package | MSIX for Windows Desktop; current manifest minimum is Windows 10 version 1809 (`10.0.17763.0`). |

The MSIX currently declares `runFullTrust`; describe the capability accurately in certification notes if Partner Center requests it. Enter the reserved Partner Center identity name, publisher, publisher display name, and four-part MSIX version into the packaging workflow; do not invent these values.

## Required assets and review inputs

Use the six current dark-mode screenshots under `website/assets/screenshots/` as content references, not necessarily as final store assets. Capture clean final-build images after store signing/packaging and show the real app, not marketing mock-ups.

Suggested sequence:

1. Home — choose a project / resume a paused session.
2. Active timer — pause, resume, and end a session.
3. Activity note — note plus manual time adjustment.
4. Evaluation — period selector and project totals.
5. Project detail — editable completed entries.
6. Export — CSV, Markdown, and JSON choices.
7. Project management — archive and restore behaviour.

For Apple, provide one to ten screenshots per localisation in JPG or PNG without alpha; use the current macOS screenshot specifications for the target display size. For Microsoft, one screenshot is required per store listing and four or more are recommended. The Windows submission also needs markets, audience, discoverability, base price, category, age-rating answers, and a validated MSIX package.

## Launch blockers — do not mark the stores ready yet

1. **Apple distribution:** add App Sandbox entitlements, migrate/validate the database location for sandboxing, sign with Mac App Distribution, use a provisioning profile, and validate the final build on a clean macOS account. The current non-notarised GitHub-style macOS build is not an App Store artifact.
2. **Public legal/support information:** replace the website privacy-policy and imprint placeholders, publish real support contact information, and expose those links inside the app. Apple requires a privacy-policy URL and support URL for the macOS listing.
3. **Privacy declaration:** verify the final signed builds and every included dependency/SDK, then answer the Apple App Privacy questionnaire accurately. “No data collected” is plausible from the current code, but it is a final legal/product declaration, not an assumption.
4. **Windows package identity and certification:** reserve the product name, supply final Partner Center identity values to the MSIX workflow, build the package, run package validation and the Windows App Certification Kit, then test a clean Windows installation.
5. **Metadata consistency:** reconcile `Tempo` versus the current Windows manifest name `Tempo Task Tracker`; confirm legal rights-holder/copyright, final price, territories, and supported Windows versions before publication.
6. **Final evidence:** replace development screenshots with screenshots from the final signed packages, test the fully offline flow, export failure handling, backup/restore on macOS, localisation, system appearance, relaunch/recovery, and the website URLs.

## Official submission references checked 17 September 2026

- Apple’s [app information reference](https://developer.apple.com/help/app-store-connect/reference/app-information/app-information) sets the name/subtitle limits and requires a privacy-policy URL for macOS.
- Apple’s [platform-version reference](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information) specifies required localized description, keywords, support URL, copyright, screenshots, and reviewer information; its [privacy guidance](https://developer.apple.com/help/app-store-connect/manage-app-information/manage-app-privacy) requires an accurate App Privacy declaration.
- Microsoft’s [MSIX submission checklist](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission) lists pricing/availability, properties, age rating, package, and Store listing requirements. Its [listing guidance](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/add-and-edit-store-listing-info) specifies a required description, one required screenshot, and recommends four or more.
