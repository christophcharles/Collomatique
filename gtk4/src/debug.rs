use std::path::PathBuf;
use std::time::Instant;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DebugMode {
    Help,
    CheckerRecon,
    CheckerBlame,
    CheckerBlameMax,
    CheckerSolve,
    FullRecon,
    FullBlame,
    FullBlameMax,
    FullSolve,
    Objective,
}

pub fn print_help() -> Result<(), anyhow::Error> {
    eprintln!("Available debug modes:");
    eprintln!();
    eprintln!("  help               Show this help message");
    eprintln!();
    eprintln!("  Checker modes (user constraints + constraint-needed extras only):");
    eprintln!(
        "    checker-recon      Reconstruct extra variables from current colloscope (CBC logging on)"
    );
    eprintln!(
        "    checker-blame      List violated constraints after reconstruction (minimal filtering)"
    );
    eprintln!("    checker-blame-max  List ALL violated constraints without filtering");
    eprintln!("    checker-solve      Solve the checker ILP from scratch (CBC logging on)");
    eprintln!();
    eprintln!("  Full modes (all constraints + objectives):");
    eprintln!("    full-recon         Reconstruct all extra variables (CBC logging on)");
    eprintln!(
        "    full-blame         List violated constraints after full reconstruction (minimal filtering)"
    );
    eprintln!("    full-blame-max     List ALL violated constraints without filtering");
    eprintln!("    full-solve         Solve the full ILP from scratch (CBC logging on)");
    eprintln!();
    eprintln!("  Other:");
    eprintln!(
        "    objective          Compute the objective function value for the current colloscope"
    );
    eprintln!();
    eprintln!("  All modes except 'help' require a file argument.");
    eprintln!("  Blame modes use 'minimal' filtering by default: redundant constraints implied");
    eprintln!("  by more specific ones are removed. Use 'blame-max' variants to see everything.");
    Ok(())
}

pub fn run(mode: DebugMode, file: PathBuf) -> Result<(), anyhow::Error> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let t_total = Instant::now();

        let t = Instant::now();
        eprintln!("Loading file: {:?}", file);
        let (data, _caveats) = collomatique_storage::load_data_from_file(&file).await?;
        let inner_data = data.get_inner_data().clone();
        eprintln!("  File loaded in {:.2?}", t.elapsed());

        eprintln!("Building ILP model...");
        let pool = sqlx::SqlitePool::connect(":memory:").await?;
        collomatique_sqlite_state::create_schema(&pool).await?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data).await?;
        let model = collomatique_constraints_colloscopes::build_model_with_log(&pool, &mut |msg| {
            eprintln!("  {msg}")
        })
        .await;
        let stats = model.stats();
        eprintln!("  Model statistics:");
        eprintln!("    Base variables: {}", stats.base_variable_count);
        eprintln!("    User constraints: {}", stats.user_constraint_count);
        eprintln!(
            "    Constraint extras: {} ({} defining constraints)",
            stats.constraint_extra_count, stats.constraint_defining_constraint_count,
        );
        eprintln!(
            "    Objective extras: {} ({} defining constraints)",
            stats.objective_extra_count, stats.objective_defining_constraint_count,
        );

        match mode {
            DebugMode::Help => unreachable!("handled before file loading"),
            DebugMode::CheckerRecon => recon(&model, &inner_data, true),
            DebugMode::FullRecon => recon(&model, &inner_data, false),
            DebugMode::CheckerBlame => blame(&model, &inner_data, true, true),
            DebugMode::CheckerBlameMax => blame(&model, &inner_data, true, false),
            DebugMode::FullBlame => blame(&model, &inner_data, false, true),
            DebugMode::FullBlameMax => blame(&model, &inner_data, false, false),
            DebugMode::CheckerSolve => solve(&model, true),
            DebugMode::FullSolve => solve(&model, false),
            DebugMode::Objective => objective(&model, &inner_data),
        }

        eprintln!("Total: {:.2?}", t_total.elapsed());
        Ok(())
    })
}

fn recon(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
    checker: bool,
) {
    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running {label} reconstruction (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    match sol {
        Some(_) => eprintln!("  Reconstruction SUCCEEDED in {:.2?}", t.elapsed()),
        None => eprintln!("  Reconstruction FAILED in {:.2?}", t.elapsed()),
    }
}

fn blame(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
    checker: bool,
    minimal: bool,
) {
    use collomatique_constraints_colloscopes::ConstraintSource;

    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running {label} reconstruction (silent)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
    let sol = if checker {
        model.checker_solution_from_data(&config_data, &solver)
    } else {
        model.solution_from_data(&config_data, &solver)
    };

    let Some(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug {label}-recon' to diagnose.",
            t.elapsed()
        );
        return;
    };
    eprintln!("  Reconstruction succeeded in {:.2?}", t.elapsed());

    let t = Instant::now();
    let filter_label = if minimal { "minimal" } else { "all" };
    eprintln!("Checking constraint violations ({filter_label})...");
    let env = &inner_data.params;
    let violations: Vec<_> = if minimal {
        solution
            .minimal_blame()
            .iter()
            .map(|desc| desc.user_readable(env))
            .collect()
    } else {
        solution
            .blame()
            .filter_map(|(_constraint, desc)| match desc {
                ConstraintSource::User(desc) => Some(desc.user_readable(env)),
                ConstraintSource::DefiningExtra { .. } => None,
            })
            .collect()
    };

    if violations.is_empty() {
        eprintln!("  All user constraints satisfied ({:.2?})", t.elapsed());
    } else {
        eprintln!(
            "  {} constraint(s) violated ({:.2?}):",
            violations.len(),
            t.elapsed()
        );
        for (i, msg) in violations.iter().enumerate() {
            eprintln!("    [{}] {}", i + 1, msg);
            if i >= 49 {
                eprintln!("    ... ({} more)", violations.len() - 50);
                break;
            }
        }
    }
}

fn solve(model: &collomatique_constraints_colloscopes::ColloscopeModel, checker: bool) {
    let label = if checker { "checker" } else { "full" };

    let t = Instant::now();
    eprintln!("Solving {label} ILP (CBC logging enabled)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(false);
    let sol = if checker {
        model.solve_checker(&solver)
    } else {
        model.solve(&solver)
    };

    match sol {
        Some(_) => eprintln!("  Solve SUCCEEDED in {:.2?}", t.elapsed()),
        None => eprintln!(
            "  Solve FAILED (no feasible solution) in {:.2?}",
            t.elapsed()
        ),
    }
}

fn objective(
    model: &collomatique_constraints_colloscopes::ColloscopeModel,
    inner_data: &collomatique_state_colloscopes::InnerData,
) {
    let t = Instant::now();
    eprintln!("Building config from current colloscope...");
    let config_data = collomatique_constraints_colloscopes::convert::build_complete_config(
        &inner_data.params,
        &inner_data.colloscope,
    );
    eprintln!("  Config built in {:.2?}", t.elapsed());

    let t = Instant::now();
    eprintln!("Running full reconstruction (silent)...");
    let solver = collomatique_ilp::solvers::coin_cbc::CbcSolver::with_disable_logging(true);
    let sol = model.solution_from_data(&config_data, &solver);

    let Some(solution) = sol else {
        eprintln!(
            "  Reconstruction failed in {:.2?}. \
             Use '--debug full-recon' to diagnose.",
            t.elapsed()
        );
        return;
    };
    eprintln!("  Reconstruction succeeded in {:.2?}", t.elapsed());

    let t = Instant::now();
    let value = solution.eval();
    eprintln!("  Objective value: {value} ({:.2?})", t.elapsed());
}
