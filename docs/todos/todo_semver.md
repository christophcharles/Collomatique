# TODO: real semantic versions (and an alpha warning)

Store versions in the spec-2 files with the **`semver` crate**, and compare
them with `semver`'s own ordering, instead of the hand-rolled `Version` type in
`storage/src/json.rs:73`.

That type is three `u32` fields (`major`, `minor`, `patch`), and
`Version::current()` builds them from `CARGO_PKG_VERSION_MAJOR/MINOR/PATCH`, so
`CARGO_PKG_VERSION_PRE` is dropped on the floor. A prerelease version simply
cannot be represented today — which is the whole point here, because the
current development version should be **`0.1.0-alpha.0.99`**.

Points to settle when doing it:

- The serialized shape changes: `produced_with_version` goes from a record of
  three numbers to a version string. The header is `deny_unknown_fields`, so
  this is a real format change and needs a reading path for existing files.
  It is mitigated by `produced_with_version` being **informational only** — it
  never gates readability (see `docs/file_format/file_format.md`, §1
  "Versioning") — so nothing depends on its ordering yet.
- `minimum_spec_version` is a separate, plain integer and is what actually
  gates readability. It should stay as it is; this todo is only about the
  application version.

Independently of the file format: add a **warning dialog on startup** saying
this is (pre)alpha software. It should state plainly that files may break and
that results should be checked, and it must appear unconditionally rather than
being tied to a "first run" flag.
