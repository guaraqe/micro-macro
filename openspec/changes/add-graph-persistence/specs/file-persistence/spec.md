# File Persistence

## ADDED Requirements

### Requirement: Auto-load state.json on startup
The application SHALL automatically load graph state from `state.json` if it exists in the working directory.

#### Scenario: Load existing state.json on startup
**Given** a file named `state.json` exists in the working directory
**When** the application starts
**Then** the graph state is loaded from `state.json`
**And** both dynamical system and observable graphs reflect the saved state

#### Scenario: Use default state when state.json missing
**Given** no `state.json` file exists in the working directory
**When** the application starts
**Then** the default state is created via the explicit default state function
**And** the graphs are initialized with default values

#### Scenario: Handle corrupted state.json
**Given** `state.json` exists but contains invalid JSON
**When** the application starts
**Then** the application logs an error or shows a message
**And** falls back to default state
**And** the application remains functional

### Requirement: Manual save to user-chosen file
The application SHALL allow users to manually save the current graph to a JSON file of their choice.

#### Scenario: Save dynamical system via file picker
**Given** the Dynamical System tab is active
**When** the user selects File > Save from the menu
**Then** a native file picker dialog opens
**And** the user can choose a filename and location
**And** the dynamical system graph is saved to the selected file

#### Scenario: Save observable via file picker
**Given** the Observable tab is active
**When** the user selects File > Save from the menu
**Then** a native file picker dialog opens
**And** the user can choose a filename and location
**And** the observable graph is saved to the selected file

#### Scenario: Handle save errors
**Given** a file system error occurs during save (e.g., permission denied, disk full)
**When** the save operation fails
**Then** the system displays an error dialog with the error message
**And** the application remains in a stable state

#### Scenario: Cancel save dialog
**Given** the save file picker dialog is open
**When** the user cancels the dialog
**Then** no file is written
**And** the current graph state remains unchanged

### Requirement: Manual load from user-chosen file
The application SHALL allow users to manually load a graph from any JSON file.

#### Scenario: Load dynamical system from file
**Given** the Dynamical System tab is active
**When** the user selects File > Load from the menu
**Then** a native file picker dialog opens filtered to .json files
**And** the user can select a file
**And** the dynamical system graph is replaced with the loaded graph
**And** the layout is reset to display the new graph

#### Scenario: Load observable from file
**Given** the Observable tab is active
**When** the user selects File > Load from the menu
**Then** a native file picker dialog opens filtered to .json files
**And** the user can select a file
**And** the observable graph is replaced with the loaded graph
**And** the layout is reset to display the new graph

#### Scenario: Handle load errors
**Given** a file cannot be read or is invalid
**When** the load operation fails
**Then** the system displays an error dialog explaining the issue
**And** the current graph state remains unchanged

#### Scenario: Cancel load dialog
**Given** the load file picker dialog is open
**When** the user cancels the dialog
**Then** no file is loaded
**And** the current graph state remains unchanged

### Requirement: Menu-based file operations
The application SHALL provide a menu bar with file operations.

#### Scenario: Display file menu
**Given** the application is running
**When** rendering the top panel
**Then** a menu bar is visible with a "File" menu
**And** the File menu contains "Save" and "Load" options

#### Scenario: File menu works in both tabs
**Given** the user is on either tab (Dynamical System or Observable)
**When** the user opens the File menu
**Then** both Save and Load options are available and functional
