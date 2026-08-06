# TODO: let the advanced tools disable soft objectives

Add a way to **disable** the soft (balancing) objectives, exposed in the
"advanced tools". Today they can only be made harder, never turned off.

The `Balancing` block of the spec-2 format
(`docs/file_format/file_format.md`, §4.14) carries five booleans, globally and
per subject. Each goal is *always* active: `true` enforces it as a strict
constraint, `false` leaves it as an optimisation goal. There is no third
value meaning "do not pursue this at all", and that missing third value is
what this todo is about.

Persistence is probably a **new block** rather than a change to `Balancing`.
Turning those booleans into a three-valued enum would rewrite the meaning of an
existing block, whereas a new block is additive: an older reader skips it
through `minimum_spec_version` / `needed_entry` and still reads the document.
The alternative (the enum) is cleaner conceptually and should be weighed
against that compatibility cost when the piece is planned.

The UI side is a panel in the advanced tools, next to the existing balancing
options, with the same global/per-subject split so a subject can opt out of a
goal the rest of the document keeps.
