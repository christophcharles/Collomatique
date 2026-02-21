use std::collections::HashMap;
use std::fmt::Write;

use collomatique_ilp::{
    Problem, UsableData, Variable, f64_is_zero, linexpr::EqSymbol, mat_repr::ProblemRepr,
    objectives::ObjectiveSense,
};

const MAX_NAME_LEN: usize = 100;

pub struct MpsNames<V: UsableData> {
    pub variables: HashMap<V, String>,
    pub constraints: Vec<String>,
    pub objective: String,
}

fn sanitize_name(debug_str: &str) -> String {
    let stripped = debug_str
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(debug_str);

    let mut result = String::with_capacity(stripped.len());
    let mut prev_underscore = false;

    for c in stripped.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_underscore = false;
        } else {
            if !prev_underscore {
                result.push('_');
                prev_underscore = true;
            }
        }
    }

    // Trim trailing underscore
    while result.ends_with('_') {
        result.pop();
    }
    // Trim leading underscore
    while result.starts_with('_') {
        result.remove(0);
    }

    if result.is_empty() {
        "_".to_string()
    } else {
        result
    }
}

/// Truncate a sanitized (ASCII-only) name, trimming trailing underscores.
fn truncate_name(name: &str, max_len: usize) -> &str {
    if name.len() <= max_len {
        return name;
    }
    let mut end = max_len;
    while end > 0 && name.as_bytes()[end - 1] == b'_' {
        end -= 1;
    }
    if end == 0 {
        &name[..max_len.min(name.len())]
    } else {
        &name[..end]
    }
}

pub fn generate_names<V, C, P>(problem: &Problem<V, C, P>) -> MpsNames<V>
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    // Variable names: sorted by Debug representation for determinism
    let mut var_entries: Vec<_> = problem
        .get_variables()
        .keys()
        .map(|v| (v.clone(), format!("{:?}", v)))
        .collect();
    var_entries.sort_by(|a, b| a.1.cmp(&b.1));

    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut variables = HashMap::new();

    const UNIQUENESS_BUDGET: usize = 10;
    let var_max = MAX_NAME_LEN.saturating_sub(UNIQUENESS_BUDGET);
    for (v, debug_str) in var_entries {
        let sanitized = sanitize_name(&debug_str);
        let base = truncate_name(&sanitized, var_max);
        let name = make_unique(base, &mut seen);
        variables.insert(v, name);
    }

    // Constraint names: c{index}_{sanitized_description}
    let constraints: Vec<String> = problem
        .get_constraints()
        .iter()
        .enumerate()
        .map(|(i, (_, desc))| {
            let prefix = format!("c{}_", i);
            let desc_budget = MAX_NAME_LEN.saturating_sub(prefix.len());
            let sanitized = sanitize_name(&format!("{:?}", desc));
            let truncated = truncate_name(&sanitized, desc_budget);
            format!("{}{}", prefix, truncated)
        })
        .collect();

    MpsNames {
        variables,
        constraints,
        objective: "obj".to_string(),
    }
}

fn make_unique(base: &str, seen: &mut HashMap<String, usize>) -> String {
    let count = seen.entry(base.to_string()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base.to_string()
    } else {
        format!("{}_{}", base, count)
    }
}

pub fn generate_mps<V, C, P>(problem: &Problem<V, C, P>, names: &MpsNames<V>) -> String
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    let mut out = String::new();

    write_name_section(&mut out);
    write_objsense_section(&mut out, problem.get_objective().get_sense());
    write_rows_section(&mut out, problem, names);
    write_columns_section(&mut out, problem, names);
    write_rhs_section(&mut out, problem, names);
    write_bounds_section(&mut out, problem, names);
    writeln!(out, "ENDATA").unwrap();

    out
}

fn write_name_section(out: &mut String) {
    writeln!(out, "NAME          problem").unwrap();
}

fn write_objsense_section(out: &mut String, sense: ObjectiveSense) {
    writeln!(out, "OBJSENSE").unwrap();
    match sense {
        ObjectiveSense::Minimize => writeln!(out, "    MIN").unwrap(),
        ObjectiveSense::Maximize => writeln!(out, "    MAX").unwrap(),
    }
}

fn write_rows_section<V, C, P>(out: &mut String, problem: &Problem<V, C, P>, names: &MpsNames<V>)
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    writeln!(out, "ROWS").unwrap();
    writeln!(out, " N  {}", names.objective).unwrap();

    for (i, (constraint, _)) in problem.get_constraints().iter().enumerate() {
        let row_type = match constraint.get_symbol() {
            EqSymbol::LessThan => "L",
            EqSymbol::Equals => "E",
        };
        writeln!(out, " {}  {}", row_type, names.constraints[i]).unwrap();
    }
}

fn write_columns_section<V, C, P>(out: &mut String, problem: &Problem<V, C, P>, names: &MpsNames<V>)
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    // Pre-build index: variable -> [(constraint_index, coefficient)]
    let mut var_constraint_map: HashMap<&V, Vec<(usize, f64)>> = HashMap::new();
    for (i, (constraint, _)) in problem.get_constraints().iter().enumerate() {
        for (v, coef) in constraint.get_lhs().coefficients() {
            if !f64_is_zero(coef) {
                var_constraint_map.entry(v).or_default().push((i, coef));
            }
        }
    }

    // Objective coefficients
    let obj_func = problem.get_objective().get_function();

    // Sort variables by MPS name for deterministic output
    let mut sorted_vars: Vec<_> = names.variables.iter().collect();
    sorted_vars.sort_by_key(|(_, name)| name.as_str());

    // Partition into continuous and integer variables
    let problem_vars = problem.get_variables();
    let (continuous, integer): (Vec<_>, Vec<_>) = sorted_vars
        .into_iter()
        .partition(|(v, _)| !problem_vars[*v].is_integer());

    writeln!(out, "COLUMNS").unwrap();

    // Emit continuous variables first
    for (v, mps_name) in continuous {
        write_column_entries(out, v, mps_name, obj_func, &var_constraint_map, names);
    }

    // Emit integer variables inside marker block
    if !integer.is_empty() {
        writeln!(out, "    INTMARK    'MARKER'                 'INTORG'").unwrap();
        for (v, mps_name) in integer {
            write_column_entries(out, v, mps_name, obj_func, &var_constraint_map, names);
        }
        writeln!(out, "    INTMARK    'MARKER'                 'INTEND'").unwrap();
    }
}

fn write_column_entries<V: UsableData>(
    out: &mut String,
    v: &V,
    mps_name: &str,
    obj_func: &collomatique_ilp::LinExpr<V>,
    var_constraint_map: &HashMap<&V, Vec<(usize, f64)>>,
    names: &MpsNames<V>,
) {
    // Objective coefficient
    if let Some(coef) = obj_func.get(v.clone()) {
        if !f64_is_zero(coef) {
            writeln!(out, "    {}  {}  {}", mps_name, names.objective, coef).unwrap();
        }
    }

    // Constraint coefficients
    if let Some(entries) = var_constraint_map.get(v) {
        for &(ci, coef) in entries {
            writeln!(out, "    {}  {}  {}", mps_name, names.constraints[ci], coef).unwrap();
        }
    }
}

fn write_rhs_section<V, C, P>(out: &mut String, problem: &Problem<V, C, P>, names: &MpsNames<V>)
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    writeln!(out, "RHS").unwrap();

    for (i, (constraint, _)) in problem.get_constraints().iter().enumerate() {
        let rhs = -constraint.get_constant();
        if !f64_is_zero(rhs) {
            writeln!(out, "    rhs  {}  {}", names.constraints[i], rhs).unwrap();
        }
    }
}

fn write_bounds_section<V, C, P>(out: &mut String, problem: &Problem<V, C, P>, names: &MpsNames<V>)
where
    V: UsableData,
    C: UsableData,
    P: ProblemRepr<V>,
{
    let problem_vars = problem.get_variables();

    // Sort by MPS name for deterministic output
    let mut sorted_vars: Vec<_> = names.variables.iter().collect();
    sorted_vars.sort_by_key(|(_, name)| name.as_str());

    let mut has_bounds = false;

    for (v, mps_name) in &sorted_vars {
        let var = &problem_vars[*v];
        let bounds_lines = format_bounds(var, mps_name);
        if !bounds_lines.is_empty() {
            if !has_bounds {
                writeln!(out, "BOUNDS").unwrap();
                has_bounds = true;
            }
            for line in bounds_lines {
                writeln!(out, "{}", line).unwrap();
            }
        }
    }
}

fn format_bounds(var: &Variable, name: &str) -> Vec<String> {
    let is_int = var.is_integer();
    let min = var.get_min();
    let max = var.get_max();

    // Binary: integer with min=0, max=1
    if is_int {
        if let (Some(lo), Some(hi)) = (min, max) {
            if f64_is_zero(lo) && f64_is_zero(hi - 1.0) {
                return vec![format!(" BV bnd  {}", name)];
            }
        }
    }

    // Free: no min, no max
    if min.is_none() && max.is_none() {
        return vec![format!(" FR bnd  {}", name)];
    }

    // Default MPS: 0 <= x < +inf (continuous)
    // Skip if matches default
    if !is_int {
        if let (Some(lo), None) = (min, max) {
            if f64_is_zero(lo) {
                return vec![];
            }
        }
    }

    let mut lines = Vec::new();

    match min {
        None => {
            // MI: lower bound is -inf
            lines.push(format!(" MI bnd  {}", name));
        }
        Some(lo) if !f64_is_zero(lo) => {
            lines.push(format!(" LO bnd  {}  {}", name, lo));
        }
        _ => {} // min = 0, default
    }

    if let Some(hi) = max {
        lines.push(format!(" UP bnd  {}  {}", name, hi));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_ilp::{
        ProblemBuilder,
        linexpr::LinExpr,
        objectives::{Objective, ObjectiveSense},
    };

    fn var(name: &str) -> LinExpr<String> {
        LinExpr::var(name.to_string())
    }

    fn cst(value: f64) -> LinExpr<String> {
        LinExpr::constant(value)
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("hello"), "hello");
        assert_eq!(sanitize_name("\"hello\""), "hello");
        assert_eq!(sanitize_name("hello world"), "hello_world");
        assert_eq!(sanitize_name("a--b..c"), "a_b_c");
        assert_eq!(sanitize_name("\"\""), "_");
        assert_eq!(sanitize_name("abc_def"), "abc_def");
        assert_eq!(sanitize_name("___"), "_");
        assert_eq!(sanitize_name("a___b"), "a_b");
    }

    #[test]
    fn test_make_unique() {
        let mut seen = HashMap::new();
        assert_eq!(make_unique("x", &mut seen), "x");
        assert_eq!(make_unique("y", &mut seen), "y");
        assert_eq!(make_unique("x", &mut seen), "x_2");
        assert_eq!(make_unique("x", &mut seen), "x_3");
    }

    #[test]
    fn test_simple_problem() {
        // min x + y subject to x + y <= 10, x = unsigned integer, y = continuous
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("x".to_string(), Variable::uinteger())
            .set_variable("y".to_string(), Variable::non_negative())
            .add_constraint(
                (var("x") + var("y")).leq(&cst(10.0)),
                "sum_limit".to_string(),
            )
            .set_objective(Objective::new(
                var("x") + var("y"),
                ObjectiveSense::Minimize,
            ))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert!(mps.contains("NAME"));
        assert!(mps.contains("OBJSENSE"));
        assert!(mps.contains("MIN"));
        assert!(mps.contains("ROWS"));
        assert!(mps.contains("COLUMNS"));
        assert!(mps.contains("RHS"));
        assert!(mps.contains("ENDATA"));
        assert!(mps.contains("'INTORG'"));
        assert!(mps.contains("'INTEND'"));
    }

    #[test]
    fn test_binary_variables() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("a".to_string(), Variable::binary())
            .set_variable("b".to_string(), Variable::binary())
            .add_constraint(
                (var("a") + var("b")).leq(&cst(1.0)),
                "at_most_one".to_string(),
            )
            .set_objective(Objective::new(
                var("a") + var("b"),
                ObjectiveSense::Maximize,
            ))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert!(mps.contains("MAX"));
        assert!(mps.contains(" BV bnd"));
        let bv_count = mps.lines().filter(|l| l.contains("BV bnd")).count();
        assert_eq!(bv_count, 2);
    }

    #[test]
    fn test_equality_constraint() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("x".to_string(), Variable::non_negative())
            .add_constraint(var("x").eq(&cst(5.0)), "fix_x".to_string())
            .set_objective(Objective::new(var("x"), ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert!(mps.contains(" E  c0_fix_x"));
    }

    #[test]
    fn test_free_variable() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("x".to_string(), Variable::continuous())
            .add_constraint(var("x").leq(&cst(10.0)), "upper".to_string())
            .set_objective(Objective::new(var("x"), ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert!(mps.contains(" FR bnd"));
    }

    #[test]
    fn test_deterministic_output() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("z".to_string(), Variable::binary())
            .set_variable("a".to_string(), Variable::binary())
            .set_variable("m".to_string(), Variable::binary())
            .add_constraint(
                (var("z") + var("a") + var("m")).leq(&cst(2.0)),
                "limit".to_string(),
            )
            .set_objective(Objective::new(
                1.0 * var("z") + 2.0 * var("a") + 3.0 * var("m"),
                ObjectiveSense::Minimize,
            ))
            .build()
            .unwrap();

        let names1 = generate_names(&problem);
        let mps1 = generate_mps(&problem, &names1);

        let names2 = generate_names(&problem);
        let mps2 = generate_mps(&problem, &names2);

        assert_eq!(mps1, mps2);
    }

    #[test]
    fn test_objective_constant_ignored() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("x".to_string(), Variable::non_negative())
            .add_constraint(var("x").leq(&cst(10.0)), "limit".to_string())
            .set_objective(Objective::new(var("x") + 42.0, ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        // The constant 42 should not appear as a coefficient in COLUMNS
        for line in mps.lines() {
            if line.contains("obj") && line.contains("42") {
                panic!(
                    "Objective constant should not appear in MPS output: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_mixed_variable_types() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("bin".to_string(), Variable::binary())
            .set_variable("int".to_string(), Variable::integer().min(0.0).max(10.0))
            .set_variable("cont".to_string(), Variable::non_negative())
            .set_variable("free".to_string(), Variable::continuous())
            .add_constraint(
                (var("bin") + var("int") + var("cont") + var("free")).leq(&cst(20.0)),
                "total".to_string(),
            )
            .set_objective(Objective::new(
                1.0 * var("bin") + 2.0 * var("int") + 3.0 * var("cont") + 4.0 * var("free"),
                ObjectiveSense::Minimize,
            ))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert!(mps.contains(" BV bnd"));
        assert!(mps.contains(" FR bnd"));
        assert!(mps.contains(" UP bnd"));
        assert!(mps.contains("'INTORG'"));
        assert!(mps.contains("'INTEND'"));
    }

    #[test]
    fn test_rhs_values() {
        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable("x".to_string(), Variable::non_negative())
            .set_variable("y".to_string(), Variable::non_negative())
            .add_constraint((var("x") + var("y")).leq(&cst(5.0)), "limit".to_string())
            .set_objective(Objective::new(var("x"), ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        let rhs_line = mps
            .lines()
            .find(|l| l.contains("rhs") && l.contains("c0_limit"))
            .expect("Should have RHS line for constraint");
        assert!(
            rhs_line.contains("5"),
            "RHS line should contain 5: {}",
            rhs_line
        );
    }

    #[test]
    fn test_scheduling_problem() {
        let slots = vec!["s1", "s2", "s3", "s4"];
        let teachers = vec!["tA", "tB"];

        let mut builder: ProblemBuilder<String, String> = ProblemBuilder::new();
        let mut obj = cst(0.0);

        for t in &teachers {
            for s in &slots {
                let var_name = format!("{}_{}", t, s);
                builder = builder.set_variable(var_name.clone(), Variable::binary());
                obj = obj + LinExpr::var(var_name);
            }
        }

        for t in &teachers {
            let expr: LinExpr<String> = slots
                .iter()
                .map(|s| LinExpr::var(format!("{}_{}", t, s)))
                .fold(cst(0.0), |acc, e| acc + e);
            builder = builder.add_constraint(expr.leq(&cst(2.0)), format!("{}_max", t));
        }

        for s in &slots {
            let expr: LinExpr<String> = teachers
                .iter()
                .map(|t| LinExpr::var(format!("{}_{}", t, s)))
                .fold(cst(0.0), |acc, e| acc + e);
            builder = builder.add_constraint(expr.leq(&cst(1.0)), format!("{}_max", s));
        }

        builder = builder.set_objective(Objective::new(obj, ObjectiveSense::Maximize));

        let problem = builder.build().unwrap();
        let names = generate_names(&problem);
        let mps = generate_mps(&problem, &names);

        assert_eq!(names.variables.len(), 8);
        assert_eq!(names.constraints.len(), 6);

        assert!(mps.contains("NAME"));
        assert!(mps.contains("MAX"));
        assert!(mps.contains("ROWS"));
        assert!(mps.contains("COLUMNS"));
        assert!(mps.contains("RHS"));
        assert!(mps.contains("BOUNDS"));
        assert!(mps.contains("ENDATA"));

        let bv_count = mps.lines().filter(|l| l.contains("BV bnd")).count();
        assert_eq!(bv_count, 8);
    }

    #[test]
    fn test_truncate_name() {
        assert_eq!(truncate_name("hello", 10), "hello");
        assert_eq!(truncate_name("hello", 5), "hello");
        assert_eq!(truncate_name("hello_world", 5), "hello");
        assert_eq!(truncate_name("hello_world", 6), "hello");
        // Trailing underscores after truncation are trimmed
        assert_eq!(truncate_name("abc___def", 5), "abc");
        assert_eq!(truncate_name("abc___def", 6), "abc");
        // Edge case: all underscores
        assert_eq!(truncate_name("______", 3), "___");
    }

    #[test]
    fn test_long_names_are_truncated() {
        let long_name = "a".repeat(200);
        let long_desc = "d".repeat(200);

        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable(long_name.clone(), Variable::binary())
            .add_constraint(LinExpr::var(long_name).leq(&cst(1.0)), long_desc)
            .set_objective(Objective::new(cst(0.0), ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);

        for name in names.variables.values() {
            assert!(
                name.len() <= MAX_NAME_LEN,
                "Variable name too long ({} chars): {}",
                name.len(),
                name,
            );
        }
        for name in &names.constraints {
            assert!(
                name.len() <= MAX_NAME_LEN,
                "Constraint name too long ({} chars): {}",
                name.len(),
                name,
            );
        }
    }

    #[test]
    fn test_truncated_variables_remain_unique() {
        // Two variables that differ only past the truncation point
        let prefix = "x".repeat(95);
        let var1 = format!("{}aaaaa", prefix);
        let var2 = format!("{}bbbbb", prefix);

        let problem: Problem<String, String> = ProblemBuilder::new()
            .set_variable(var1.clone(), Variable::binary())
            .set_variable(var2.clone(), Variable::binary())
            .add_constraint(
                (LinExpr::var(var1) + LinExpr::var(var2)).leq(&cst(1.0)),
                "c".to_string(),
            )
            .set_objective(Objective::new(cst(0.0), ObjectiveSense::Minimize))
            .build()
            .unwrap();

        let names = generate_names(&problem);
        let name_values: Vec<&String> = names.variables.values().collect();

        assert_eq!(name_values.len(), 2);
        assert_ne!(name_values[0], name_values[1], "Names should be unique");
    }
}
