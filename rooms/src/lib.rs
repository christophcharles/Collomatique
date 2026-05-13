pub mod parsing;

use std::path::Path;
use std::time::Instant;

pub use collomatique_rooms_model::{
    Config, DemandConflict, DemandConflictKind, DemandKind, Hour, Incompat,
    InterrogationRoomPreference, Periods, PrepRoomPreference, Request, Room, ScheduleData,
    TimeZone, Window,
};
pub use parsing::{RoomPreferenceWarning, ScheduleError};

pub fn run(
    rooms: &Path,
    requests: &Path,
    incompats: Option<&Path>,
    checker_only: bool,
    config: Config,
    timeout_minutes: u32,
) -> Result<(), ScheduleError> {
    let (data, pref_warnings) = parsing::parse_schedule(rooms, requests, incompats, config)?;
    eprintln!(
        "Parsed {} rooms, {} requests, and {} incompatibilities",
        data.rooms.len(),
        data.requests.len(),
        data.incompats.len(),
    );
    for w in &pref_warnings {
        match w {
            RoomPreferenceWarning::Redundancy {
                row,
                column,
                room,
                original_entries,
                merged_result,
            } => {
                eprintln!(
                    "Warning: request row {row}, column \"{column}\": room \"{room}\" \
                     specified multiple times ({entries}), merged to {merged_result}",
                    entries = original_entries.join(", "),
                );
            }
            RoomPreferenceWarning::InterrogationAndPrepWithoutSharing { row, room } => {
                eprintln!(
                    "Warning: request row {row}: room \"{room}\" appears in both Salle and Prep \
                     but is not marked for sharing (+). Did you mean to add + to enable sharing?",
                );
            }
            RoomPreferenceWarning::ConflictingPreferences {
                row,
                room,
                positive_entries,
                negative_entries,
            } => {
                eprintln!(
                    "Error: request row {row}, column \"Salle\": room \"{room}\" has both \
                     positive ({pos}) and negative ({neg}) preferences",
                    pos = positive_entries.join(", "),
                    neg = negative_entries.join(", "),
                );
            }
        }
    }
    let has_conflicts = pref_warnings
        .iter()
        .any(|w| matches!(w, RoomPreferenceWarning::ConflictingPreferences { .. }));
    let unreg = data.unregistered_rooms();
    for name in &unreg.demanded {
        eprintln!(
            "Warning: room \"{name}\" is not registered in the rooms file. \
             In case of double occupancy, we will not be able to find the closest available room."
        );
    }
    for name in &unreg.suggested {
        eprintln!(
            "Error: room \"{name}\" is suggested but not registered in the rooms file. \
             Cannot determine location for proximity matching."
        );
    }
    for conflict in data.demand_conflicts() {
        print_demand_conflict(&data, &conflict);
    }
    if has_conflicts {
        let rooms: Vec<String> = pref_warnings
            .iter()
            .filter_map(|w| match w {
                RoomPreferenceWarning::ConflictingPreferences { room, .. } => Some(room.clone()),
                RoomPreferenceWarning::Redundancy { .. }
                | RoomPreferenceWarning::InterrogationAndPrepWithoutSharing { .. } => None,
            })
            .collect();
        return Err(ScheduleError::ConflictingRoomPreferences(rooms));
    }
    if !unreg.suggested.is_empty() {
        return Err(ScheduleError::UnregisteredSuggestedRooms(
            unreg.suggested.iter().map(|s| s.to_string()).collect(),
        ));
    }

    eprintln!("Building ILP model...");
    let start = Instant::now();
    let model = collomatique_constraints_rooms::build_model(&data);
    let elapsed = start.elapsed();
    let stats = model.stats();
    eprintln!(
        "  {} base variables, {} constraints (built in {:.2?})",
        stats.base_variable_count, stats.user_constraint_count, elapsed,
    );

    if checker_only {
        eprintln!("Solving (checker only, no objective)...");
    } else {
        eprintln!("Solving...");
    }
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(false);
    let solved = if timeout_minutes == 0 {
        if checker_only {
            model.solve_checker(&solver).map(|s| s.get_data())
        } else {
            model.solve(&solver).map(|s| s.get_data())
        }
    } else {
        use collomatique_ilp::solvers::{Solver, TimeLimitSolverModel};
        let solve_with_timeout = move |pb| {
            let result = solver
                .build_model(pb)
                .solve_with_time_limit(timeout_minutes * 60);
            if result.time_limit_reached {
                eprintln!("Warning: solver time limit ({timeout_minutes} min) reached.");
            }
            result.config
        };
        if checker_only {
            model
                .solve_checker_with(solve_with_timeout)
                .map(|s| s.get_data())
        } else {
            model.solve_with(solve_with_timeout).map(|s| s.get_data())
        }
    };
    match solved {
        Some(config) => {
            let assignments = collomatique_constraints_rooms::extract_assignments(&data, &config);
            for assignment in &assignments {
                let req = &data.requests[assignment.request];
                let room_str: &str = assignment.room.as_ref();
                let subjects_str: String = req
                    .subjects
                    .iter()
                    .map(|s| s.as_ref() as &str)
                    .collect::<Vec<&str>>()
                    .join(";");
                if let Some(prep) = &assignment.prep_room {
                    let prep_str: &str = prep.as_ref();
                    println!(
                        "Request {}: {} {}h {} — Room: {}, Prep: {}",
                        assignment.request, req.day, *req.hour, subjects_str, room_str, prep_str,
                    );
                } else {
                    println!(
                        "Request {}: {} {}h {} — Room: {}",
                        assignment.request, req.day, *req.hour, subjects_str, room_str,
                    );
                }
            }
            eprintln!("Solved: {} assignments", assignments.len());
        }
        None => {
            eprintln!("No feasible solution found.");
        }
    }

    Ok(())
}

fn print_demand_conflict(data: &ScheduleData, conflict: &DemandConflict) {
    let room: &str = conflict.room.as_ref();
    match &conflict.kind {
        DemandConflictKind::InterrogationInterrogation => {
            eprintln!(
                "Warning: room \"{room}\" demanded for interrogation \
                 by conflicting requests on {} at {}:",
                conflict.day, conflict.hour,
            );
            for &(req_idx, _) in &conflict.requests {
                print_demand_request(data, req_idx, "interrogation");
            }
        }
        DemandConflictKind::InterrogationPrep {
            can_share_with_prep,
        } => {
            if *can_share_with_prep {
                eprintln!(
                    "Warning: room \"{room}\" demanded for interrogation (with prep sharing) \
                     and prep on {} at {} — might not conflict if capacity allows:",
                    conflict.day, conflict.hour,
                );
            } else {
                eprintln!(
                    "Warning: room \"{room}\" demanded for both interrogation \
                     and prep on {} at {}:",
                    conflict.day, conflict.hour,
                );
            }
            for &(req_idx, ref kind) in &conflict.requests {
                let label = match kind {
                    DemandKind::Interrogation => "interrogation",
                    DemandKind::Prep => "prep",
                };
                print_demand_request(data, req_idx, label);
            }
        }
        DemandConflictKind::PrepOverCapacity {
            total_students,
            capacity,
        } => {
            eprintln!(
                "Warning: prep demands for room \"{room}\" on {} at {} \
                 exceed capacity ({total_students} students for {capacity} seats):",
                conflict.day, conflict.hour,
            );
            for &(req_idx, _) in &conflict.requests {
                print_prep_demand_request(data, req_idx);
            }
        }
        DemandConflictKind::PrepUnknownCapacity { total_students } => {
            eprintln!(
                "Warning: multiple prep demands for unlisted room \"{room}\" \
                 on {} at {} ({total_students} students total, capacity unknown):",
                conflict.day, conflict.hour,
            );
            for &(req_idx, _) in &conflict.requests {
                print_prep_demand_request(data, req_idx);
            }
        }
    }
}

fn format_subjects(req: &Request) -> String {
    req.subjects
        .iter()
        .map(|s| s.as_ref() as &str)
        .collect::<Vec<&str>>()
        .join(";")
}

fn print_demand_request(data: &ScheduleData, request: usize, kind: &str) {
    let req = &data.requests[request];
    eprintln!(
        "  - Request {request} ({kind}): {}, teacher: {}, requester: {}",
        format_subjects(req),
        req.teacher.as_ref() as &str,
        req.requester.as_ref() as &str,
    );
}

fn print_prep_demand_request(data: &ScheduleData, request: usize) {
    let req = &data.requests[request];
    eprintln!(
        "  - Request {request} ({} prep students): {}, teacher: {}, requester: {}",
        req.prep_students,
        format_subjects(req),
        req.teacher.as_ref() as &str,
        req.requester.as_ref() as &str,
    );
}
