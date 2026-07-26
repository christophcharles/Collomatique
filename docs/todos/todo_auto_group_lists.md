# TODO: Automatic group-list generation

At some point we must tackle the **"Générer des listes automatiquement"** button
in `gtk4/src/editor/group_lists.rs`. This is a *huge* piece of work.

We need:

- A new crate `constraints-groups/` — this is the big part.
- A corresponding configuration UI, parallel to
  `gtk4/src/editor/colloscope/config_dialog.rs`.
- All the GUI wiring (and potentially the Python wiring).
