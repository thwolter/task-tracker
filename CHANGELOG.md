## [0.4.0] - 2026-09-19

### 🚀 Features

- *(macos)* Activate app on tray restore and update dependencies
- *(about)* Add About dialog and info icon in UI with macOS support
- *(windows)* Implement activation signaling and event listener for app window

### 🎨 Styling

- *(imports)* Reorder imports for consistency across modules

### ⚙️ Miscellaneous Tasks

- *(release)* V0.4.0
## [0.3.0] - 2026-09-17

### 🚀 Features

- *(ci)* Add OS-specific release asset preparation and include Windows icon
- Implement project deletion functionality across UI and backend
- *(tests)* Add i-slint-backend-testing dependency and test module
- *(build)* Relocate Windows icon and update resources in build script
- *(i18n)* Add translation support across UI components and views
- *(ui)* Introduce ScrollableColumn for consistent scrolling experience
- *(ui)* Update AppWindow title and button states based on tracking status
- *(language)* Add localization support and language-aware reports
- *(domain)* Modularize domain logic and add tests for data handling
- *(live-preview)* Add live preview feature and update dependencies
- *(evaluation)* Implement project-specific task evaluation view
- *(ui)* Display completed task count in evaluation view
- *(ui)* Enhance TaskCard styling and layout adjustments
- *(evaluation)* Add UI for updating and deleting completed tasks
- *(ui)* Add color palette reference and IconButton component integration
- *(ui)* Enhance evaluation tasks UI with editing and translations
- *(ui)* Implement project editor with enhanced project management
- *(ui)* Introduce new proposed color palette for evaluation
- *(macos_menu)* Add macOS native menu for data backup and restore
- *(window_state)* Add persistent window state management
- *(onboarding)* Implement first-run project creation and management
- *(ui)* Add project name validation and existing name check
- *(ui)* Add focus to note input and improve focus handling in TextEdit
- *(macos)* Enhance macOS menu with custom window attributes and update version to 0.3.0
- *(ui)* Implement Adjust Time View and enhance task interval editing
- *(instance-lock)* Add process-wide startup lock to ensure single instance
- *(dependencies)* Update `slint` to 1.18.0 and refine Cargo.lock
- *(macos)* Integrate macOS menu-bar timer with UI actions
- *(windows)* Add packaging scripts and AppxManifest for Windows MSIX
- *(ci)* Add platform input to package workflow configuration
- *(windows)* Add runFullTrust capability to AppxManifest.xml

### 🐛 Bug Fixes

- *(main)* Ensure consistent language selection in live preview
- *(ui)* Correct scrollbar logic in ScrollableColumn component
- *(translations)* Update German translations for consistency

### 📚 Documentation

- *(tracker)* Add comprehensive doc comments to tracker components
- Add Tempo release roadmap to TODO.md
- *(ui)* Add comprehensive documentation for UiController functions
- *(tracking)* Add module-level documentation for tracking controller
- *(website)* Add detailed product and legal content for store preparation

### 🚜 Refactor

- *(ui)* Replace SettingsProjectRow with ProjectListItem component
- *(domain)* Remove serde dependencies and add documentation comments
- *(ui)* Optimize DurationLabel usage and improve tooltips formatting
- *(ui)* Restructure TaskCard and enhance editing functionality
- *(ui)* Merge TaskDetails into TaskCard for streamlined logic
- *(ui)* Rename EvaluationTasks to Drilldown for improved clarity
- *(ui)* Update method names and refresh logic in task handling
- *(ui)* Remove bindings module and consolidate command handling in controller
- *(ui)* Extract AppActions and UiCommand to dedicated file
- *(ui)* Simplify dispatch calls by removing duplicate fields
- *(theme)* Update Theme properties and usage for clarity and consistency
- *(ui)* Adjust scrollbar properties and layout in ScrollableColumn
- *(ui)* Refine typography and palette structure for consistency
- Streamline project management and refresh UI styling
- *(ui)* Remove unused UI controller and tests for improved clarity
- *(ui)* Update property names for clarity and consistency
- *(ui)* Remove forward-focus and unused keyboard focus elements
- *(ui)* Extract ProjectList component for cleaner settings view
- *(ui)* Modularize project editor components for clarity and reusability
- *(ui)* Remove accessibility attributes from Slint components
- *(ui)* Update component imports and adjust layouts for consistency
- *(ui)* Adjust layout properties for consistency and clarity
- *(ui)* Extract TextEdit component for reuse and update NoteView layout
- *(ui)* Rename components and update layout for task editor and project row
- *(ui)* Streamline TaskEditor and update task card layout components
- *(ui)* Add DeleteConfirmation component and integrate delete logic
- *(ui)* Adjust layout and properties for TaskCard and buttons
- *(ui)* Streamline task editor callbacks and update task navigation
- *(ui)* Update AppWindow title to include active task and elapsed time
- *(ui)* Remove old UI wiring and enhance tracking integration
- *(tracker)* Implement interrupted task handling and improve UX
- *(duration-label)* Use Math.floor for hour calculation
- *(ui)* Enhance AppButton component with styles, variants, and intents
- *(website)* Remove proposed palette and update tempo.pot file
- *(ui)* Remove CommonStrings and inline translations
- *(theme)* Implement system-adaptive color schemes and improve tooltips
- *(export)* Unify export formats and improve modularity
- *(ui)* Enhance IconButton with intents, sizes, and styles
- *(tests)* Optimize test structure and improve task naming consistency
- *(ui)* Conditionally render VerticalLayout and Rectangle in tracking view
- *(ui)* Update AppWindow title logic for paused state display
- *(ui)* Streamline ProjectEditor logic and refactor deletion confirmation
- *(ui)* Enhance text handling in timer display and update translations
- *(ui)* Validate project name length and update error handling
- *(translations)* Remove obsolete line numbers from German translation file
- *(translations)* Update German translation entries in export view
- *(bundle)* Simplify icon structure in Cargo.toml
- *(ui)* Move backup and restore strings to NativeMenuStrings
- *(tests)* Update project name in assertion for clarity
- *(build)* Update Windows icon path and include new icon file
- *(main)* Simplify language initialization logic
- *(tray)* Unify macOS and Desktop tray logic under desktop_tray module
- *(windows)* Remove redundant MakeAppx validate step in packaging script

### 🎨 Styling

- *(ui)* Improve indentation and formatting in Slint files
- *(ui)* Improve formatting in project-editor.slint and remove redundant width/height in settings.slint
- *(imports)* Reorder imports for better code readability
- *(windows)* Unify path separators in AppxManifest.xml
- *(imports)* Reorder macOS-related imports for readability

### 🧪 Testing

- *(ui)* Add tests for evaluation task callbacks and tracker operations
## [0.1.0-rc.1] - 2026-09-06

### 🚀 Features

- Implement MVP
- *(ui)* Add UI components and views for task management
- *(app)* Implement core application modules and logic
- *(ui)* Enhance project handling and introduce new UI components
- *(application)* Implement project archiving and unarchiving features
- *(ui)* Enhance project dialog with unarchive and delete options
- *(ui, application)* Add pause functionality to task tracking
- Update tracking state
- *(ui, application)* Add project totals to evaluation view
- *(build)* Update dependencies and add project metadata in Cargo.toml
- *(build)* Rename app to Tempo and add Linux build tasks
- Create rust.yml
- *(ci)* Add package workflow and system dependencies installation
- *(ci)* Enhance Windows build and artifact upload in workflows
- *(website)* Add initial Tempo landing page and styles
- *(website)* Add Datenschutz and Impressum pages with styling updates
- *(website)* Add macOS launch instructions and styling for download page

### 📚 Documentation

- Add comprehensive README with usage, setup, and screenshots

### 🚜 Refactor

- *(ui)* Rename TaskDialog to ProjectDialog and update related components
- *(ui)* Consolidate typography components and update project dialog handling
- *(ui)* Restructure evaluation view and components for improved layout and functionality
- *(ui)* Update navigation and remove skip note functionality
- *(application)* Simplify error handling by using custom error types
- *(application)* Switch from tasks to projects for consistency
- *(ui)* Update status handling with structured Status and StatusKind
- *(ui)* Streamline project dialog management and page navigation
- *(ui)* Restructure and modularize UI controller and bindings
- *(application)* Migrate JSON persistence to SQLite database

### ⚙️ Miscellaneous Tasks

- *(ci)* Update GitHub Actions to use latest versions of actions
- *(release)* Update version to 0.1.0-rc.1 in Cargo files
