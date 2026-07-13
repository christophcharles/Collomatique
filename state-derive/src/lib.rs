mod entity_id;
mod join;
mod references;

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
