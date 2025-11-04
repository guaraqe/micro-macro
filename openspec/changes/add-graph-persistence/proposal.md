# Proposal: Add Graph Persistence

## Summary
Add save/load functionality to persist dynamical system graphs and observables to disk, with automatic loading of `state.json` on startup and manual save to user-chosen files.

## Motivation
Currently, all graph data exists only in memory and is lost when the application closes. Users cannot:
- Save their work and resume later
- Build a library of interesting dynamical systems to study
- Share graph definitions with others
- Recover from accidental closures

This significantly limits the tool's utility for serious exploration of dynamical systems.

## Scope
This change adds:
1. **File format**: JSON-based serialization of graphs (both dynamical system and observable graphs)
2. **Auto-load on startup**: If `state.json` exists in the working directory, load it automatically
3. **Default state**: Explicit default graph state function when no `state.json` exists
4. **Manual save**: Save current graph to user-chosen .json file via file picker dialog
5. **Manual load**: Load graph from user-chosen .json file via file picker dialog
6. **UI controls**: Menu bar with File > Save and File > Load options

**Out of scope** (for this change):
- Auto-save functionality
- File format versioning
- Recent files list
- Multiple file formats
- Cloud storage or sync

## Dependencies
- No dependencies on other changes
- Builds on existing serde serialization infrastructure

## Risks & Mitigations
1. **Risk**: File I/O errors (permissions, disk full, invalid files)
   **Mitigation**: Proper error handling with user-facing error messages

2. **Risk**: Large graphs could produce very large files
   **Mitigation**: JSON is text-based and compresses well; accept this for initial implementation

## Success Criteria
- Application automatically loads `state.json` on startup if it exists
- Application creates default state via explicit function when `state.json` doesn't exist
- User can manually save to any .json file via file picker
- User can manually load from any .json file via file picker
- All graph structure (nodes, edges, labels) is preserved
- Both dynamical system and observable graphs are persisted
- Clear error messages if save/load fails
- Cargo build passes after implementation
