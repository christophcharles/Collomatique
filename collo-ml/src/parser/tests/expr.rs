use super::*;

// =============================================================================
// EXPRESSION GRAMMAR TESTS
// =============================================================================
// These tests validate the SYNTACTIC structure of expressions.
// Expressions are the core of the language and include:
// - Literals (numbers, booleans, paths)
// - Arithmetic operations (+, -, *, //, %)
// - Collections (lists, comprehensions, set operations)
// - Comparisons (==, !=, <, >, <=, >=, ===, <==, >==)
// - Logical operations (and, or, not)
// - Aggregations (sum, forall)
// - Control flow (if-else)
// - Function/variable calls
// - Type annotations (as)
// - Cardinality (|expr|)
//
// These are grammar tests only - they do NOT validate semantic correctness.

// Expression test submodules
mod aggregations;
mod arithmetic;
mod cardinality;
mod collections;
mod comparisons;
mod complex;
mod control_flow;
mod literals;
mod logical;
mod semantic_invalid;
mod type_annotations;
