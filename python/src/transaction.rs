//! Grouping a block of writes into one undo slot
//!
//! Reached as `doc.transaction(label)`, and meant to be used as a `with` block
//! (`docs/python/new_api_design.md` §5). The nesting is done by
//! `collomatique_state::SessionStack`; this file is only the one-shot object
//! that opens a session on the way in and closes it on the way out.

use pyo3::prelude::*;

use collomatique_ops::OpCategory;

use crate::Document;
use crate::errors::Error;

/// Where a transaction is in its one-shot life
enum Stage {
    /// Built by `doc.transaction(...)` and never entered: it holds no session,
    /// and leaving it does nothing
    Fresh,
    /// Entered: a session is open, and it is the `depth`-th on the document
    Open { depth: usize },
    /// Committed or cancelled: leaving it again does nothing, and it cannot be
    /// entered a second time
    Closed,
}

impl Stage {
    /// What [Transaction::__repr__] shows
    fn name(&self) -> &'static str {
        match self {
            Stage::Fresh => "not entered",
            Stage::Open { .. } => "open",
            Stage::Closed => "closed",
        }
    }
}

/// A block of writes, on its way to becoming one undo slot
///
/// `doc.transaction(label)` builds one; entering it opens the block and
/// leaving it closes it. It is one-shot: once closed it cannot be entered
/// again, and a second transaction is a second call to `doc.transaction`.
#[pyclass(module = "collomatique")]
pub struct Transaction {
    doc: Py<Document>,
    label: String,
    stage: Stage,
}

impl Transaction {
    /// Builds the object — [Document::transaction] is the only way to get one
    ///
    /// Nothing happens to the document here: a transaction that is never
    /// entered never opens a session.
    pub(crate) fn new(doc: Py<Document>, label: String) -> Transaction {
        Transaction {
            doc,
            label,
            stage: Stage::Fresh,
        }
    }
}

/// Refuses to close a transaction that is not the innermost one open
///
/// `with` blocks close in the order they opened, so this only fires when a
/// script drives the two ends by hand — `outer.cancel()` from inside an inner
/// block above all. Closing the outer one there would take the inner block's
/// writes with it and leave that block holding a session that is gone.
fn check_innermost(doc: &Document, depth: usize) -> PyResult<()> {
    if doc.transaction_depth() != depth {
        return Err(Error::new_err(
            "this transaction is not the innermost one open: transactions must be \
             closed in the order they were opened",
        ));
    }
    Ok(())
}

#[pymethods]
impl Transaction {
    /// Opens the block, and hands the transaction back
    ///
    /// So `with doc.transaction("x") as t:` binds the transaction, which is
    /// what [Transaction::cancel] is reached through.
    ///
    /// Entering one twice raises `Error`: a transaction is one block, and the
    /// second `with` would be asking for a block that has no end of its own.
    fn __enter__(slf: Py<Self>, py: Python<'_>) -> PyResult<Py<Self>> {
        let doc = {
            let this = slf.borrow(py);
            match this.stage {
                Stage::Fresh => {}
                Stage::Open { .. } => {
                    return Err(Error::new_err(
                        "this transaction is already open; call doc.transaction(...) again \
                         for a second block",
                    ));
                }
                Stage::Closed => {
                    return Err(Error::new_err(
                        "this transaction is closed; call doc.transaction(...) again for a \
                         second block",
                    ));
                }
            }
            this.doc.clone_ref(py)
        };

        let depth = {
            let mut doc = doc.borrow_mut(py);
            doc.begin_transaction();
            doc.transaction_depth()
        };

        slf.borrow_mut(py).stage = Stage::Open { depth };
        Ok(slf)
    }

    /// Closes the block: commits it when it ended normally, rolls it back when
    /// an exception is leaving it
    ///
    /// It never swallows the exception — it always answers `False`, so whatever
    /// was raised carries on out of the block.
    ///
    /// Leaving a block that [Transaction::cancel] already closed does nothing,
    /// which is what makes cancelling early and then carrying on work.
    fn __exit__(
        slf: Py<Self>,
        py: Python<'_>,
        exc_type: Option<Bound<'_, PyAny>>,
        _exc_value: Option<Bound<'_, PyAny>>,
        _traceback: Option<Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let (doc, depth, label) = {
            let this = slf.borrow(py);
            match this.stage {
                Stage::Open { depth } => (this.doc.clone_ref(py), depth, this.label.clone()),
                // Never entered, or closed early by `cancel()`: there is no
                // session to close, and saying so would be noise.
                Stage::Fresh | Stage::Closed => return Ok(false),
            }
        };

        {
            let mut doc = doc.borrow_mut(py);
            check_innermost(&doc, depth)?;
            if exc_type.is_some() {
                doc.cancel_transaction();
            } else {
                // The same shape the application uses for a whole script run:
                // no category, because there is no screen a script's block
                // belongs to.
                doc.commit_transaction((OpCategory::None, label));
            }
        }

        slf.borrow_mut(py).stage = Stage::Closed;
        Ok(false)
    }

    /// Rolls the block back now, without leaving it
    ///
    /// Everything written since the block opened is taken back at once, and the
    /// transaction is closed, so leaving the block adds nothing. The block
    /// itself keeps running: writes made after this land outside the
    /// transaction, in the level it was opened from.
    ///
    /// This is how a script previews a change and then throws it away:
    ///
    /// ```python
    /// with doc.transaction("preview") as t:
    ///     doc.periods.set_first_week(candidate)
    ///     ok = looks_right(doc)
    ///     t.cancel()
    /// ```
    ///
    /// Cancelling a transaction that was never entered, or that is already
    /// closed, raises `Error`: there is nothing to take back, and doing nothing
    /// quietly would hide a script that lost track of its own blocks.
    fn cancel(slf: Py<Self>, py: Python<'_>) -> PyResult<()> {
        let (doc, depth) = {
            let this = slf.borrow(py);
            match this.stage {
                Stage::Open { depth } => (this.doc.clone_ref(py), depth),
                Stage::Fresh => {
                    return Err(Error::new_err(
                        "this transaction was never entered, so it has nothing to cancel; \
                         use it as `with doc.transaction(...)`",
                    ));
                }
                Stage::Closed => {
                    return Err(Error::new_err(
                        "this transaction is already closed, so it has nothing to cancel",
                    ));
                }
            }
        };

        {
            let mut doc = doc.borrow_mut(py);
            check_innermost(&doc, depth)?;
            doc.cancel_transaction();
        }

        slf.borrow_mut(py).stage = Stage::Closed;
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "<collomatique.Transaction {:?} ({})>",
            self.label,
            self.stage.name()
        )
    }
}
