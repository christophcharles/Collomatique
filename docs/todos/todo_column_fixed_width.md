# TODO: fixed-width columns in the other list panels

Piece 13 gave the group-list rows a **fixed** name column (commit `f411390a`,
`gtk4/src/editor/group_lists/group_lists_display.rs`): ellipsize at the end,
`width-chars` and `max-width-chars` both pinned, full name in an unconditional
`#[watch]`ed tooltip. The other list panels have the same defect and want the
same treatment.

They all size their columns with `set_size_request`, which is a *minimum*, so
one long value widens its row and every column after it stops lining up with
the other rows. The columns holding free user text:

- `gtk4/src/widgets/contact_list.rs` (shared by **Colleurs** and **Élèves**) —
  the name (200) and telephone (120) columns.
- `gtk4/src/editor/slots/slots_display.rs` — teacher name and week-pattern name
  (200 each). The start time is a bounded format and can stay a minimum.
- `gtk4/src/editor/settings.rs` — the student name (200).
- `gtk4/src/editor/balancing.rs` — the subject name (200). Its
  global-parameters row uses the same width for a fixed label, so the two must
  move together.

To settle: whether the character widths are chosen per column or shared. 200 px
is about 26 characters, but a telephone number and a subject name do not want
the same value.
