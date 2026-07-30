mod content_ord;
mod entity_id;
mod join;
mod references;

use syn::{DeriveInput, parse_macro_input};

/// Derive macro for typed ID newtypes.
///
/// Generates the `collomatique_state::ids::Id` implementation and the
/// leaf `collomatique_state::refs::References<K>` implementation
/// (for any `K: From<Self>`) for a tuple struct wrapping a single `u64`.
///
/// The optional `#[entity(Type)]` attribute declares which entity type the
/// ID points to and additionally generates the leaf join implementations:
/// `collomatique_state::join::Joinable` (with `Output<'a> = &'a Type` and
/// `Error = Self` — the dangling ID is the diagnostic) and
/// `collomatique_state::join::Join<Ctx>` for any
/// `Ctx: Lookup<Self, Entity = Type>`.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
/// #[entity(Teacher)]
/// pub struct TeacherId(u64);
/// ```
#[proc_macro_derive(EntityId, attributes(entity))]
pub fn derive_entity_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    entity_id::derive(input)
}

/// Derive macro for [`collomatique_state::refs::References`].
///
/// Walks every field annotated with `#[fk]` in declaration order and
/// forwards to that field's own `References<K>` implementation, so IDs,
/// `Option`/`Vec`/`BTreeSet` of IDs, and nested `#[derive(References)]`
/// structs all compose. At least one `#[fk]` field is required.
///
/// `#[fk(name = ident)]` is accepted (the rename belongs to the `Join`
/// derive) and treated like a plain `#[fk]`.
///
/// # Example
///
/// ```ignore
/// #[derive(References)]
/// pub struct Slot {
///     #[fk]
///     pub teacher_id: TeacherId,
///     pub start_time: SlotStart,
///     #[fk]
///     pub week_pattern: Option<WeekPatternId>,
/// }
/// ```
#[proc_macro_derive(References, attributes(fk))]
pub fn derive_references(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    references::derive(input)
}

/// Derive macro for [`collomatique_state::join::Join`].
///
/// Generates a joined view struct (fields borrow from the join context
/// with lifetime `'a`), the `collomatique_state::join::Joinable`
/// implementation pointing at it, and a `Join<Ctx>` implementation that
/// resolves every `#[fk]` field through its own `Join` implementation —
/// so IDs, `Option`/`Vec`/`BTreeSet` of IDs, and nested `#[derive(Join)]`
/// structs all compose. Non-`#[fk]` fields appear borrowed (`&'a T`) in
/// the joined struct. At least one `#[fk]` field is required.
///
/// Attributes:
/// - `#[join(error = Type)]` (mandatory): the error type; it must be
///   `From<…>` each `#[fk]` field's own error type.
/// - `#[join(output = Name)]` (optional): the joined struct's name,
///   defaulting to `Joined{Name}`.
/// - `#[fk(name = ident)]` (optional, per field): renames the joined
///   field, which otherwise keeps the source field's name.
///
/// # Example
///
/// ```ignore
/// #[derive(Join)]
/// #[join(error = NewId)]
/// pub struct Slot {
///     #[fk(name = teacher)]
///     pub teacher_id: TeacherId,       // joined as `teacher: &'a Teacher`
///     pub start_time: SlotStart,       // joined as `start_time: &'a SlotStart`
///     #[fk]
///     pub week_pattern: Option<WeekPatternId>,
/// }
/// ```
#[proc_macro_derive(Join, attributes(fk, join))]
pub fn derive_join(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    join::derive(input)
}

/// Derive macro for [`collomatique_state::partial_order::ContentOrd`].
///
/// Implements the document order as the **product of all fields**, in
/// declaration order: `Equal` iff every field is `Equal`, `Less` iff at
/// least one field is `Less` and none is `Greater` or incomparable, `None`
/// as soon as one field is incomparable or two fields disagree in
/// direction. For an enum, two values of the same variant compare as the
/// product of that variant's fields (a unit variant is the empty product,
/// `Equal`), and two values of *different* variants are incomparable.
///
/// The macro walks every field by construction, so a forgotten field is
/// impossible and a new field whose type has no `ContentOrd` impl is a
/// compile error that forces a decision.
///
/// Field attributes (at most one per field), overriding the default
/// per-type dispatch:
/// - `#[ord(atom)]`: compare discretely, inline (`==` or incomparable) —
///   the escape hatch for foreign types the orphan rule keeps out of the
///   trait. The field's type must be `Eq`.
/// - `#[ord(ignore)]`: the order does not see this field. Introduces
///   equivalence classes on purpose (an id issuer, a test-harness mode).
/// - `#[ord(with = <expr>)]`: compare with the given expression, callable
///   as `fn(&T, &T) -> Option<Ordering>`; a path or an inline closure.
/// - `#[ord(total)]`: the field's native *total* order is its content
///   order (`Ord::cmp`). Never a default — std's container orders are
///   lexicographic, which is wrong for a removal order. The field's `Ord`
///   must itself be well-founded (no infinite strictly-descending chain):
///   integers are, `String` is **not** (`"b" > "ab" > "aab" > …`) — a
///   non-well-founded `total` field silently voids the termination proof.
///   This cannot be checked mechanically; it is part of the field's design
///   decision.
///
/// Only non-generic structs with named fields and non-generic enums with
/// named-field or unit variants are supported.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Debug, ContentOrd)]
/// pub struct Data {
///     #[ord(ignore)]
///     id_issuer: IdIssuer,
///     inner_data: InnerData,
/// }
/// ```
#[proc_macro_derive(ContentOrd, attributes(ord))]
pub fn derive_content_ord(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    content_ord::derive(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for [`collomatique_state::partial_order::ContentIdentity`].
///
/// Asserts that `==` coincides with content equivalence for this type, so
/// containers may match it by `==`/`Ord`. Deliberately explicit and never
/// automatic: the compiler strips `#[derive(...)]` lists from macro input,
/// so no macro can know whether `PartialEq` is derived or hand-written, and
/// a macro must never sign a claim it cannot check.
///
/// The derive checks everything checkable — `#[ord(ignore)]` is rejected
/// (an ignored field is a content quotient by definition), `#[ord(with =
/// ...)]` is rejected as unanalyzable (hand-write the marker impl if the
/// custom rule preserves identity), default fields must themselves be
/// `ContentIdentity`, atom fields must be `Eq`, and `total` fields are free
/// by `Ord`'s own contract. The one premise it cannot verify is that this
/// type's `==` is the structural, field-wise equality — so request the
/// marker in the same derive list as a *derived* `PartialEq`.
///
/// Registers `attributes(ord)` too: the derive must read the `#[ord(...)]`
/// attributes to reject quotients, and registration keeps the attribute
/// inert even when `ContentIdentity` is derived alone.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Debug, PartialEq, Eq, ContentOrd, ContentIdentity)]
/// struct TimeWindow {
///     #[ord(total)]
///     start: u32,
///     #[ord(total)]
///     duration: u32,
/// }
/// ```
#[proc_macro_derive(ContentIdentity, attributes(ord))]
pub fn derive_content_identity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    content_ord::derive_identity(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
