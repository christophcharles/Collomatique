# Transactions and the manager stack

`docs/python/new_api_design.md` §5 promises this:

```python
with doc.transaction("Import Pronote"):
    ...                    # any number of ops → ONE undo slot
                           # exception → everything rolled back
```

and adds one line about how it is built: "Backed by `AppSession` (nestable)."

That line hides a real problem. This note writes the problem down, rules out the answer
everyone reaches for first, and describes the two designs that actually work — so that the
session which implements `transaction()` starts from here instead of deriving it again.

Nothing here is implemented. Undo and redo (`doc.undo()`, `doc.redo()`, `doc.can_undo`,
`doc.can_redo`, `doc.undo_name`, `doc.redo_name`) do *not* depend on any of this and land
separately: they are `Manager` methods on the `AppState` the document already holds, and
their shape is the same whichever design below is chosen.

---

## 1. The problem

`AppSession` (`state/src/state.rs:100`) does nest. But it nests **in the type**:

```
depth 0:  AppState<Data, Desc>
depth 1:  AppSession<AppState<Data, Desc>, Desc>
depth 2:  AppSession<AppSession<AppState<Data, Desc>, Desc>, Desc>
```

`AppSession::new` takes the manager by value (`state/src/state.rs:115`) and `commit` /
`cancel` hand it back (`:125`, `:135`), so each nesting level wraps the previous one in
another layer of type.

Every `with` block adds a layer. A python script chooses its depth when it runs — a helper
function that opens a transaction may be called from inside another one, and how deep that
goes is not knowable in advance. A rust struct field has one type, chosen when the code is
compiled.

So `Document.state` cannot simply be "whatever manager we are at right now". That is the
whole difficulty, and it is not about `Data`, undo history, or the ops layer.

## 2. Why a trait object is not the way out

This is the first idea anyone has, so here is why it fails. Three reasons; the second one is
decisive.

**It cannot be named from `python/`.** `Manager` is not implemented type by type. It is
blanket-implemented over a second, sealed trait:

```rust
// state/src/traits.rs:329
impl<T: private::ManagerInternal> Manager for T {}

// state/src/traits.rs:331
pub(crate) mod private {
    // :401
    pub trait ManagerInternal: Send + Sync + Clone {
        type Data: InMemoryData;
        type Desc: Description;
        ...
    }
}
```

The module is `pub(crate)`. A trait object must bind every associated type it carries,
including the ones inherited from supertraits — and `Data` and `Desc` live on
`ManagerInternal`, not on `Manager`. Writing `dyn Manager<Data = …, Desc = …>` from `python/`
means writing a binding for a trait that crate cannot name.

**It is not object-safe at all.** `ManagerInternal: Send + Sync + Clone`, and the standard
library declares `pub trait Clone: Sized`. A trait object is unsized by definition. So
`dyn ManagerInternal` would have to be `Sized` and not `Sized` at the same time.

This is the decisive one, because it holds **inside `state/` too**. Unsealing the module
would not help. `Manager` can never become a trait object, in any crate, without first
dropping `Clone` from `ManagerInternal` — and `Clone` is what `UpdateOp::dry_apply` needs
(`ops/src/lib.rs`, it clones the manager to apply on a copy).

**`dyn InMemoryData` is the wrong end of the problem.** `Data` is one concrete type at every
depth; it never varies. What varies with depth is the manager wrapping it.

## 3. Option A — a recursive manager in `state/`

Give `state/` a manager type whose recursion lives in the *value* rather than the type:

```rust
/// A manager with a stack of open sessions
///
/// [AppSession] nests, but only at the type level: each nesting is another
/// layer in the type. A caller whose nesting depth is decided at runtime —
/// a script opening `with doc.transaction(...)` inside another one — needs
/// the recursion in the value instead, which is what this is.
pub struct SessionStack<T: InMemoryData, D: Description> {
    // Option only so the node can be moved out and put back; never None
    // between calls.
    node: Option<Node<T, D>>,
}

enum Node<T: InMemoryData, D: Description> {
    Base(AppState<T, D>),
    Nested(Box<AppSession<SessionStack<T, D>, D>>),
}
```

with `new(data)`, `begin()`, `commit(desc) -> bool`, `cancel() -> bool` and `depth()`.
`ManagerInternal` is implemented by delegating all four accessors to the node: `Base` to the
`AppState`, `Nested` to the `AppSession`, whose own impl already returns the session's history
rather than the parent's. `Clone`, `Send` and `Sync` come from `AppState` and `AppSession`,
which have them already.

Only `state/` can write this, because of the sealing in §2. It is about 60 lines plus tests.
`Document.state` becomes a `SessionStack`, and `begin` / `commit` / `cancel` / `depth` are all
python needs.

This gives **true nesting**: an inner block rolls back only its own writes, and an outer block
that catches the inner one's exception carries on with everything it did before intact.

## 4. Option B — transactions join, entirely inside `python/`

Keep `state/` untouched, and change what a nested `with` *means*. There is at most one
`AppSession`, plus a counter:

```rust
/// The document's state, and whether a transaction is open on it
///
/// Two variants and no recursion. Transactions join rather than nest, so
/// there is never more than one `AppSession` — `depth` counts how many
/// `with` blocks are inside it.
enum Editable {
    Idle(AppState<Data, Desc>),
    Open {
        session: AppSession<AppState<Data, Desc>, Desc>,
        depth: usize,
    },
}
```

A `with` inside a `with` does not open a second session; it bumps `depth`. Only the outermost
`__exit__` commits or cancels.

Since `dyn Manager` is impossible (§2), `Editable` cannot hand out a `&dyn Manager` for the
rest of the document to use. It gets inherent `data()`, `update()`, `undo()`, `redo()`,
`can_undo()`, … methods that each match over the two arms once.

What this delivers is everything §5 states in words: any number of writes in a block become
one undo slot, an exception anywhere in the block rolls the whole block back, and a nested
`with` works rather than raising — so a helper that opens a transaction is safe to call from
inside another one.

What it gives up:

- an inner block cannot roll back only its own part: `cancel()` anywhere cancels up to the
  outermost block;
- the inner label is ignored, the slot being named by the outermost block.

### The corner Option B cannot make clean

```python
with doc.transaction("outer"):
    try:
        with doc.transaction("inner"):
            raise Boom
    except Boom:
        pass
    # ... more writes
```

The inner block failed; the outer one caught it and carried on. With one session, "roll back
the inner block and keep the outer" is not expressible. There are only two behaviours
available, and both are wrong:

- **commit anyway** — writes from a block that raised end up in the document, which is the
  exact failure §5 exists to prevent;
- **cancel everything** — an inner `__exit__` sets a *doomed* flag which the outermost one
  honours, so a caught exception rolls back work done outside the block that raised.

Cancelling is the safer half of a bad pair, and is what Option B should do. But it is a
semantic hole, not a trade: a script author meets it once and stops trusting the mechanism.
Option A does not have this corner at all.

## 5. What does not depend on the choice

These are settled either way and should not be re-argued:

- **`__enter__` opens the transaction, not the constructor.** A `Transaction` object that is
  never entered does nothing at all.
- **`__enter__` returns the transaction**, and `t.cancel()` rolls back immediately and makes
  `__exit__` a no-op. §5 says a script previews a write by cancelling a transaction and never
  says how; this is how. Entering twice, or entering after `cancel()`, raises
  `collomatique.Error`.
- **`__exit__` returns `False`**, so the exception propagates.
- **The description is `(OpCategory::None, label)`** — `Desc` is `(OpCategory, String)`
  (`ops/src/lib.rs:66`), and this is the shape the gui already uses for a whole script run
  (`"Exécution de {path}"`, `gtk4/src/editor/run_python_script.rs:399`).
- **Inside an open transaction, `undo()` reaches back no further than the transaction's
  start.** That is `AppSession`'s own semantics and both options inherit it unchanged.
- **A document's history never leaves the script.** In hosted mode the script works on a copy,
  so an undo or a rolled-back transaction is invisible to the application; only
  `send_to_host` crosses.

Whichever is chosen, two edits to `docs/python/new_api_design.md` come with it: §5's
"nestable" sentence must be replaced by what nesting actually means, and §6 must gain
`NothingToUndo`.

## 6. Recommendation

**Option A.** It costs about 60 lines in `state/` and gives up nothing.

Option B costs nothing outside `python/`, but it buys that with the hole in §4. The
preference for leaving `state/` alone is the only reason Option B is written down here at
all — if that preference softens, Option A is the better design and there is nothing else to
weigh.

Neither is ratified. This section is a recommendation, not a decision.
