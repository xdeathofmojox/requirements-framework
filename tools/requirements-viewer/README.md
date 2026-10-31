# Requirements viewer

A local, read-only browser view of requirements and their parent links. Files are
validated before the viewer opens. No frontend build, remote service, or account
is needed.

From the framework repository root:

```sh
nix run .#requirements-viewer -- schemas/examples/traceability.json
```

Or from the development shell:

```sh
nix develop
requirements-view schemas/examples/traceability.json
```

Open several files together to trace across documents:

```sh
requirements-view requirements/system.json requirements/software.json
```

Use a custom schema to validate and display project-specific fields:

```sh
requirements-view --schema schemas/examples/project.schema.json schemas/examples/project-requirements.json
```

One schema selection applies to all input files. The default is the strict v1
schema; use `--schema-version v1` to select it explicitly. All fields, including
nested custom data, appear in the inspector. The viewer adds no new schema fields.

Select a card to inspect its data and follow parent or child links. Search matches
any requirement data. Choose **Selected trace** to focus on the selected
requirement's ancestors and descendants.

Layers are inferred from parent depth: roots are at layer 0, and a child is one
layer below its deepest parent. Layers do not imply system/software categories.
Missing or ambiguous parents and cycles leave affected layers unresolved. The
viewer reports these traceability issues without hiding records. Duplicate IDs
are reported across all input files.

The server binds to `127.0.0.1` on an available port and serves a validated snapshot.
Press Ctrl+C to stop. Restart to pick up file changes. To print a URL without
opening a browser, or choose a fixed port:

```sh
requirements-view --no-browser --port 8765 schemas/examples/traceability.json
```

The Nix package is `requirements-viewer`; the overlay exposes
`pkgs.requirements-viewer`. It depends on `requirements-validator`, which provides
the schemas. The viewer neither edits requirements nor evaluates test coverage.
