//! Simple schedule example
//!
//! This module implements a *very* simple example of scheduling problem to illustrate
//! the usage of [crate::BaseConstraints] and [crate::ExtraConstraints].
//!
//! The problem being implemented here is a very simple scheduling problem where
//! we have a few courses (let's n of them) and a few groups of students (let's
//! note m their number) than should attend them.
//!
//! The courses are supposed to happen all at the same time and all groups should attend
//! each course exactly once. That's it. There are only two variables to our problem: n and m.
//! And we only have to complete a very simple schedule.
