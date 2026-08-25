# TODO: Let a CLI script open, and save, a document

`--python` and `--python-file` run a script with no document. `main.rs` passes
`None` as the host, so `current_document()` is None and the script has to name
and load its own files. A positional `[FILE]` given alongside a script parses
fine today and is then silently ignored.

Three parts:

- **`[FILE]` opens the document.** Load it with
  `collomatique_storage::deserialize_data` and hand the script a
  `collomatique_python::Host`, so `current_document()` answers it. The trait is
  two methods, `data()` and `send()`; `RpcHost` in
  `colloscopes/rpc-engine-colloscopes/src/lib.rs` is the shape to copy — there
  `send` relays over RPC, here it would just replace what the process holds.
- **A new option saves it at the end**, `--out <PATH>` or similar, written with
  `collomatique_storage::serialize_data`. It should combine with `-n`/`--new`,
  so that a script can build a document from nothing and write it out. That
  means relaxing the clap group: `--new` is currently in the `script` group's
  `conflicts_with_all`, and `[FILE]` is not in it at all.
- **A warning when work is dropped**: no `--out`, but the document was
  modified, so say the modifications are lost. "Modified" needs no diffing — a
  script sends explicitly (`docs/python/new_api_design.md` §9.2), so the
  question is only whether `Host::send` was ever called.

Open points: what `--out` should do when it names the file that was opened
(overwrite in place, or refuse); and what to do with the caveats
`deserialize_data` returns alongside the data — the GUI shows them, a script
run has nowhere to put them but stderr.

If the warning text is French, it belongs in `colloscopes/ui-text` like the
rest of the user-facing vocabulary.
