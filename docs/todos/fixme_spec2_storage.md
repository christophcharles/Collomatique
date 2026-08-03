# FIXME: the spec-2 decoder leans on the invariant gate for diagnostics

Most semantic constraints of the file format are not checked while decoding.
The decoder rebuilds an `InnerData` and hands it to `Data::from_inner_data`,
which rejects the document — correctly, so this is not a soundness problem, but
the message the user gets is bad. It comes out of `FixableInvariant`'s
`Display`, so it is in English, in the vocabulary of the in-memory model, and it
names no block and no row. In a French application the user reads something like
*"Les données ne vérifient pas un invariant : pairing rule PairingRuleId(12)'s
antecedent names subject SubjectId(2) which has interrogations disabled"* and has
no idea which entry of the file to go and fix.

An earlier session wrote this up in `storage/src/decode.rs` as though it were
the design ("a decoder that happens to catch a problem earlier is a convenience,
not a guarantee"). It is not: the gate is a last line of defence, and every
constraint the decoder can name, it should name. Those comments are gone; the
gap they described is not.

## The goal

The end state is that **a broken invariant at the gate becomes a `panic!`**.

Once every constraint of the spec is checked while decoding, a document that
reaches `Data::from_inner_data` and fails there is not a bad file — it is a bug
in this crate, a place where the decoder built an `InnerData` it had no business
building. That deserves a panic, exactly as the `ops/` families panic on an
invariant break they did not scan for. Concretely, at the end of the audit:

- `DecodeError::BrokenInvariants` leaves the enum, and with it the
  `BrokenInvariants` arm of `error_to_string` in
  `gtk4/src/loading/file_loader.rs`. No user-facing error is ever phrased in the
  vocabulary of the in-memory model again.
- `impl From<FromInnerDataError> for DecodeError` (`storage/src/decode.rs`)
  panics on that variant instead of translating it.
- Decide at the same time what happens to `DecodeError::LogicError`
  (`FromInnerDataError::Logic`): same argument, probably the same fate, but it
  is a different check and should be looked at on its own terms rather than
  swept along.

That turns a soft rule into a hard contract: **the spec-2 decoder and the
in-memory representation must be kept in sync**. Every invariant in
`state-colloscopes/src/invariants.rs` needs a decode-time counterpart that
diagnoses it first, and any invariant added later must arrive with one — or the
panic becomes reachable from a merely-corrupt file, which would be a bad
regression rather than a caught bug. Worth deciding, when the panic lands, how
that contract is enforced: a checklist in the invariants module, a test that
walks a corpus of deliberately-broken documents, or something better.

## The work

It is an audit rather than a design: §4 of `docs/file_format/file_format.md`
already ends each block with a `Constraints:` line, and each one should be
checked against what `storage/src/decode/spec2.rs` actually diagnoses. Where it
does not, add a precise `DecodeError` variant with its French message in
`gtk4/src/loading/file_loader.rs`, plus a rejection test in
`storage/tests/spec2_format.rs`.

Known offenders include the teachers' `subjects` list (§4.3), the slots'
teacher and week-pattern references (§4.7), the incompatibilities' subject
(§4.8), the group-list associations' three ids (§4.10), the slot pairing rules'
slots (§4.12), the settings' students (§4.13) and the balancing subjects
(§4.14) — the list is indicative, the audit is the task.

`Pairings` (§4.11) is done and is the pattern to copy: see
`reconstruct_pairings`, its two `DecodeError` variants, and the five tests named
`pairing_rule_*` / `a_rule_naming_one_*`.

Worth deciding during the audit: whether a *dangling reference* deserves a
per-block variant at all, or whether one generic "block B, row R references an
unknown X" variant covers the whole family better. The per-constraint variants
that already exist (`UnknownSubjectInSlots`, `UnknownPeriodInAssignments`, …)
suggest per-block, but that is a lot of variants and the choice should be made
once, deliberately.
