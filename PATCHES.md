# Designer fork: what differs from upstream

Branch `designer`, on Graphite's `latest-stable` tag (`95c1ab81`, 2026-07-01).
The rule is new files, plus as few edits to upstream files as register them.
Every upstream file touched is listed here, with why.

## What the fork adds

A host page (the Designer dashboard) frames the editor, opens one surface of a
design in it, and takes the result back as layers. See `editor/src/designer.rs`
for the editor's side and `frontend/src/designer-bridge.ts` for the messages.

| New file | What it is |
|---|---|
| `editor/src/designer.rs` | Build a document from a design (an artboard, one labelled picture layer per design layer); describe what the document holds; render one layer alone. |
| `editor/src/designer_tests.rs` | Those three, against a headless editor (`cargo test -p graphite-editor --lib designer_tests`). |
| `frontend/wrapper/src/designer.rs` | JavaScript bindings for the above (hand-written: they return values). |
| `frontend/src/designer-bridge.ts` | The `postMessage` protocol a host speaks, and `window.designerGraphite`. |
| `designer-build.sh` | Builds the web editor into `frontend/dist`. |
| `branding/`, `designer-branding/` | Our own icons and logo, and the script that generates them (`branding/BRANDING.md`). |

## Upstream files edited

| File | Change | Why |
|---|---|---|
| `editor/src/lib.rs` | `pub mod designer;` and its test module | Registers the module. |
| `editor/src/messages/portfolio/document/utility_types/network_interface.rs` | `set_input_data()` | Upstream can read a node input's `input_data` but has no public way to write one key. Layer labels live there. |
| `frontend/wrapper/src/lib.rs` | `mod designer;` | Registers the bindings. |
| `frontend/src/App.svelte` | `installDesignerBridge(editor)` | Starts the bridge once the editor exists. |
| `frontend/src/utility-functions/files.ts` | `downloadFileBlob` asks `window.designerCapture` first | An export or save the bridge asked for goes to the bridge, not to the person's downloads. |
| `frontend/src/utility-functions/service-worker.ts` | No automatic reload when framed | The reload on a new service worker also fires on a first visit, and throws away the design a host has just opened. |
| `frontend/src/utility-functions/persistence.ts` | No autosaved documents read or written when framed | The host is where a design is kept. A copy autosaved in the editor's own storage came back as a stray tab the next time any design was opened. |
| `tools/cargo-run/src/branding.rs`, `.gitignore` | Skip the branded-asset download when `branding/BRANDING.md` exists; commit `branding/` | Graphite's logo and icons may not be used in a modified build. |

## Building

`sh designer-build.sh` (its header lists the tools). The wrapper is built with
`--no-default-features --features gpu`: the default `shader-nodes` feature
does not compile at this commit.

## Updating to a newer Graphite

Rebase `designer` onto the new commit, then run the designer tests and a build.
The parts most likely to move: `GraphOperationMessage::NewBitmapLayer` and
`NewArtboard`, `NodeGraphExecutor::submit_document_export`, the document
metadata the export copy is given (`export_layer`), and the `input_data` map.
