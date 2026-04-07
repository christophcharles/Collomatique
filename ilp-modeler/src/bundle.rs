//! Composable model fragments.
//!
//! A [`ConstraintBundle`] is "the same arguments you'd pass to
//! [`Modeler::add_constraint`], [`Modeler::add_objective`] and
//! [`Modeler::declare_extra`], queued up to be applied to a
//! modeler later." It carries eager constraints, weighted
//! objectives, and lazy extra-variable definitions, and can be
//! composed with other bundles via [`ConstraintBundle::merge`]
//! before being handed to [`Modeler::apply_bundle`].
//!
//! [`IntConstraintBundle`] is the same shape but with
//! [`IntConstraint`] in place of [`Constraint`]. It exists to
//! support integer-specific transformations (notably reification,
//! added in a follow-up). Drop the int wrapping with
//! [`IntConstraintBundle::into_general`].

use std::fmt::Debug;
use std::marker::PhantomData;

use collomatique_ilp::{Constraint, IntConstraint, Objective, UsableData, Variable};

use crate::{DefineFn, Modeler, Var};

// ---------------------------------------------------------------------------
// ExtraEntry
// ---------------------------------------------------------------------------

/// One declare-extra worth of arguments, stored as a value so
/// that it can sit inside a bundle until the bundle is applied.
pub struct ExtraEntry<'m, B, E, Db, Err>
where
    B: UsableData,
    E: UsableData,
{
    pub name: E,
    pub kind: Variable,
    pub define: Box<DefineFn<'m, B, E, Db, Err>>,
}

impl<'m, B, E, Db, Err> ExtraEntry<'m, B, E, Db, Err>
where
    B: UsableData,
    E: UsableData,
{
    /// Build an entry from a closure of the same shape as
    /// [`Modeler::declare_extra`]. The closure is boxed under
    /// the proper HRTB so callers don't have to wrangle the
    /// `dyn` lifetimes themselves.
    pub fn new<F>(name: E, kind: Variable, define: F) -> Self
    where
        F: for<'a> FnOnce(
                &'a Db,
                &'a mut crate::HelperFactory<B, E>,
                &'a crate::VarKinds<'a, B, E>,
                E,
            ) -> crate::BoxFuture<
                'a,
                Result<Vec<Constraint<crate::ExtraVar<B, E>>>, Err>,
            > + 'm,
    {
        ExtraEntry {
            name,
            kind,
            define: Box::new(define),
        }
    }
}

// ---------------------------------------------------------------------------
// ConstraintBundle
// ---------------------------------------------------------------------------

/// A queued-up set of operations to perform on a [`Modeler`].
/// The three fields shadow [`Modeler::add_constraint`],
/// [`Modeler::add_objective`] and [`Modeler::declare_extra`]
/// one-to-one. Bundles compose by appending and are consumed by
/// [`Modeler::apply_bundle`].
pub struct ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Eager user constraints.
    pub constraints: Vec<(Constraint<Var<B, E>>, C)>,
    /// Weighted objective contributions. Mirrors the modeler's
    /// internal `objectives` vec — merging two bundles is
    /// concat, no arithmetic, all weights preserved.
    pub objectives: Vec<(f64, Objective<Var<B, E>>)>,
    /// Lazy extra-variable definitions. Each entry is exactly
    /// what `declare_extra` would receive: a name, a kind, and
    /// a definition closure. Stored as closures because there is
    /// no way to flatten an extra back into top-level
    /// constraints.
    pub extras: Vec<ExtraEntry<'m, B, E, Db, Err>>,
    _phantom: PhantomData<Db>,
}

impl<'m, B, E, C, Db, Err> Default for ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        ConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Db, Err> ConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Empty bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a list of constraints with descriptions.
    pub fn from_constraints<I>(constraints: I) -> Self
    where
        I: IntoIterator<Item = (Constraint<Var<B, E>>, C)>,
    {
        ConstraintBundle {
            constraints: constraints.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras concat in order; no arithmetic.
    pub fn merge(&mut self, other: Self) {
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
    }
}

// ---------------------------------------------------------------------------
// IntConstraintBundle
// ---------------------------------------------------------------------------

/// Same shape as [`ConstraintBundle`] but the eager constraints
/// are [`IntConstraint`] rather than [`Constraint`]. The
/// objective stays as [`Objective`] (no `IntObjective`).
pub struct IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    pub constraints: Vec<(IntConstraint<Var<B, E>>, C)>,
    pub objectives: Vec<(f64, Objective<Var<B, E>>)>,
    pub extras: Vec<ExtraEntry<'m, B, E, Db, Err>>,
    _phantom: PhantomData<Db>,
}

impl<'m, B, E, C, Db, Err> Default for IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    fn default() -> Self {
        IntConstraintBundle {
            constraints: Vec::new(),
            objectives: Vec::new(),
            extras: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'m, B, E, C, Db, Err> IntConstraintBundle<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
{
    /// Empty bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a list of int constraints with descriptions.
    pub fn from_constraints<I>(constraints: I) -> Self
    where
        I: IntoIterator<Item = (IntConstraint<Var<B, E>>, C)>,
    {
        IntConstraintBundle {
            constraints: constraints.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Append all of `other`'s entries into `self`. Constraints,
    /// objectives, and extras concat in order; no arithmetic.
    pub fn merge(&mut self, other: Self) {
        self.constraints.extend(other.constraints);
        self.objectives.extend(other.objectives);
        self.extras.extend(other.extras);
    }

    /// Drop the int wrapping. Each [`IntConstraint`] is unwrapped
    /// into its underlying [`Constraint`]; objectives and extras
    /// pass through unchanged.
    pub fn into_general(self) -> ConstraintBundle<'m, B, E, C, Db, Err> {
        ConstraintBundle {
            constraints: self
                .constraints
                .into_iter()
                .map(|(c, desc)| (c.into_constraint(), desc))
                .collect(),
            objectives: self.objectives,
            extras: self.extras,
            _phantom: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Modeler::apply_bundle
// ---------------------------------------------------------------------------

impl<'m, B, E, C, Db, Err> Modeler<'m, B, E, C, Db, Err>
where
    B: UsableData,
    E: UsableData,
    C: UsableData,
    Err: Debug + 'static,
{
    /// Apply a bundle to the modeler: push every constraint,
    /// push every weighted objective, and declare every extra.
    /// Equivalent to repeated `add_constraint` / `add_objective`
    /// / `declare_extra` calls in field-then-vec order.
    pub fn apply_bundle(&mut self, bundle: ConstraintBundle<'m, B, E, C, Db, Err>) {
        for (c, desc) in bundle.constraints {
            self.add_constraint(c, desc);
        }
        for (coef, obj) in bundle.objectives {
            self.add_objective(coef, obj);
        }
        for entry in bundle.extras {
            // The boxed `define` already matches DefineFn —
            // declare_extra_boxed (in lib.rs) inserts it
            // directly without re-wrapping.
            self.declare_extra_boxed(entry.name, entry.kind, entry.define);
        }
    }
}
