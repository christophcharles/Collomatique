# TODO: Generic CSV import dialog

A generic dialog for importing CSV files — maybe even spreadsheet files — should
be designed. The current Pronote web import is nice but far too narrow.

This needs careful design. The model we have in mind so far:

- Simple **rules** that can import students, teachers and subjects, and change a
  few things along the way.
- The dialog does not import directly: it **generates a Python script**, which
  the user can then edit and save for later reuse.
- The rules are built from drop-down lists showing the columns of the CSV and
  the values found in them, so the user picks rather than types.

What is *not* clear yet is how the model should work so that a saved script
stays reusable on a different file. To be thought about.

The generated scripts are only as good as the API they drive, so this rests on
the Python API — `docs/python/new_api_design.md`.
