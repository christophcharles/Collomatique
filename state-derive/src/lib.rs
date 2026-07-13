mod entity_id;
mod references;

/// Derive macro for typed ID newtypes.
///
/// Generates the `collomatique_state::ids::Id` implementation and the
/// leaf `collomatique_state::refs::References<K>` implementation
/// (for any `K: From<Self>`) for a tuple struct wrapping a single `u64`.
///
/// The optional `#[entity(Type)]` attribute declares which entity type the
/// ID points to; it is validated but only used by the `Join` machinery.
///
/// # Example
///
/// ```ignore
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, EntityId)]
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
