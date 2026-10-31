# Requirements viewer

An Axum-based requirements viewer with a layered graph and inspector. Browser
assets are embedded in the executable.

From the framework repository root, open a read-only view:

```sh
nix run .#requirements-viewer -- schemas/examples/traceability.json
```

To add requirements to your branch's JSON files, enable additions explicitly:

```sh
nix run .#requirements-viewer -- --edit requirements/
```

You can mix individual files and folders. Folders are scanned recursively for
`.json` documents containing `requirements` or `schemaVersion`. Other valid JSON
files and `*.schema.json` files are ignored. Malformed JSON is reported, not
silently skipped. Hidden entries and nested symlinks are skipped. Overlapping
inputs are deduplicated. All discovered requirement documents must pass the
selected schema; `--edit` applies to every loaded document.

Click **Reload files** after external changes. It rescans the original folders,
picks up added, modified, or deleted documents, and validates the result before
updating the graph and save targets. A failed reload retains the previous
snapshot and shows an error. Browser refreshes only fetch the existing in-memory
snapshot; they do not reread or revalidate files. Empty folders can be opened;
use **Add requirement → Create a new document…** to create the first file, or
add a document on disk and click **Reload files** to pick it up.

Use **Add requirement**, choose a target file, and fill in the ID, title, statement,
and optional parent IDs. Selecting a requirement first prefills it as a parent.
To start a new file, choose **Create a new document…** as the target, select one
of the supplied root folders, and enter a relative `.json` path, such as
`system/controls/requirements.json`. Missing subfolders are created after validation.
Absolute paths, `..`, hidden entries, and symlink folders are rejected.
Document fields start
with `{"schemaVersion":"1.0"}`; add any document-level fields required by your
project schema. The first requirement is validated with the complete document
before the file is created. Existing files are never overwritten by creation.

To update an existing record, select its card and click **Edit requirement** in
the inspector. The form loads its statement, title, parents, and custom fields.
**Validate and save** replaces that record in its current file. The ID is fixed
to preserve incoming trace links; this editor does not rename IDs or move records
between files. Editing requires `--edit` and the same validation and conflict
checks as additions.

To delete a record, select it and click **Delete requirement** in the inspector.
A second dialog shows its ID, title, and source file. **Cancel** leaves it
unchanged; **Yes, delete requirement** performs the validated removal. This
requires `--edit`. Remove incoming parent references from other loaded
requirements first. Deletion never cascades. When deleting the last requirement, the confirmation
explicitly warns that the entire JSON file, including document metadata, will be
deleted. Otherwise, the remaining document must satisfy its schema. Reloads no
longer expect an explicitly opened file after the viewer deletes it. Review the resulting changes in Git as usual.

The form shows declared schema fields automatically, including inherited fields
and project-specific properties. Required fields have `*`; enums use dropdowns,
booleans use choices, numbers use numeric inputs, and string arrays use one value
per line. Nested objects and complex values use a labeled JSON input. Optional
fields have **Include field** so absent, false, zero, and empty values stay distinct.
Existing values are prefilled when editing.

**Additional fields** is only shown when the schema permits undeclared properties.
It accepts a JSON object and cannot override a dedicated field. With the default
strict schema or the strict project example, it is hidden. Required fields and
conditional constraints remain enforced by full validation when saving; the form
is an editing aid, not a separate schema validator. Use **Reload files** after
changing a project schema to refresh its controls.

**Validate and save** appends the new requirement, writes the file, and refreshes
the graph. Validation errors stay in the form so you can correct the record.
No Git commands run automatically. Review and commit the changes yourself:

```sh
git diff -- requirements/
git add requirements/
git commit -m "Add system requirements"
```

Use a project schema when it defines different fields or ID rules:

```sh
nix run .#requirements-viewer -- --edit \
  --schema schemas/examples/project.schema.json \
  schemas/examples/project-requirements.json
```

All opened files use the same schema selection. The default is strict v1;
`--schema-version v1` selects it explicitly. Custom data remains JSON and appears
in the inspector, including nested objects and arrays.

Save targets are loaded files or new documents inside supplied folders. Additions must pass
the selected schema and have an ID unique across the session. Existing missing
links and cycles are displayed as diagnostics; they are not schema failures.
Files are rewritten as two-space JSON, with existing values and unknown document
fields preserved. External file changes cause saves to fail; click **Reload files**
to load those changes before saving. Avoid concurrent editors or viewer sessions writing the
same files: conflict checks are optimistic, not a filesystem transaction across
independent applications. File replacement is atomic and preserves permissions.

## Appearance

The header shows a sun in light mode and a moon in dark mode. Click it to switch
between neutral gray
light and dark themes. Appearance follows your browser/OS until you toggle it.
Your choice is remembered in browser storage for this viewer address; a different
port has separate storage.

## Map layout and scope

- **Layout** switches between left-to-right columns and top-to-bottom rows.
- **View → Selected as root** shows the selection and its descendants, placing it at layer 0.
  Focused layers count the shortest parent-link distance from that root.
- **View → One layer up** shows the selection and its immediate parents.
- **View → All requirements** restores the complete map, subject to the search filter.
- **View → Selected trace** shows all ancestors and descendants of the selection.

Focused views follow the selected card. Toolbar controls are grouped into
**Find and select**, **Display**, and **Files**. Source paths appear at the bottom
of the inspector. These controls change only the view; parent links and files
are unchanged.

The server binds only to loopback. Save requests require the local origin and a
session token. To choose a port without launching a browser:

```sh
nix run .#requirements-viewer -- --no-browser --port 8765 \
  schemas/examples/traceability.json
```

The overlay provides `pkgs.requirements-viewer`. Nix supplies the existing
schema validator and schemas; Cargo dependencies are pinned in the application
manifest and the repository's workspace lockfile.

Run the Rust tests from the framework repository root:

```sh
nix develop --command cargo test --workspace --locked
```

`nix flake check` also builds the Rust package and runs these tests. Browser
interactions use vanilla JavaScript; this implementation does not add WebAssembly.

| Source | Responsibility |
| --- | --- |
| `src/main.rs` | CLI and local server startup |
| `src/document.rs` | File snapshots |
| `src/discovery.rs` | Recursive folder discovery |
| `src/graph.rs` | Parent graph and inferred layers |
| `src/store.rs` | Validated additions and file persistence |
| `src/validation.rs` | Packaged validator integration |
| `src/server.rs` | Axum routes and session guards |
| `web/` | Graph UI and addition form |
