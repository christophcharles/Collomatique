//! Matrix representation of problems and configurations
//!
//! This module contains the definition of two traits [ProblemRepr] and [ConfigRepr].
//! These traits are used to represent ILP problem and corresponding configurations into
//! some actual numerical representation, usually matrix repr.
//!
//! In addition to these traits, the module contains two submodules [nd] and [sparse]
//! which contain the implementation using the ndarray crate and a sparse matrix representation
//! respectively.
//!
//! Technically, it should be possible to represent problems and configurations using something else
//! than matrices. But this is the straightforward way to do it and only this way was indeed
//! implemented.
