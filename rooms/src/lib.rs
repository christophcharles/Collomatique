pub mod parsing;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

pub use collomatique_rooms_model::{
    Config, DemandConflict, DemandConflictKind, DemandKind, Hour, Incompat, Periods, Request, Room,
    RoomPreference, ScheduleData, TimeZone, Window,
};
pub use parsing::ScheduleError;

pub fn run(rooms: &Path, requests: &Path, incompats: Option<&Path>) -> Result<(), ScheduleError> {
    let data = parsing::parse_schedule(rooms, requests, incompats)?;
    eprintln!(
        "Parsed {} rooms, {} requests, and {} incompatibilities",
        data.rooms.len(),
        data.requests.len(),
        data.incompats.len(),
    );
    for name in data.unregistered_rooms() {
        eprintln!(
            "Warning: room \"{name}\" is not registered in the rooms file. \
             In case of double occupancy, we will not be able to find the closest available room."
        );
    }
    for conflict in data.demand_conflicts() {
        print_demand_conflict(&data, &conflict);
    }

    print_busiest_slot(&data);

    eprintln!("Building ILP model...");
    let start = Instant::now();
    let model = collomatique_constraints_rooms::build_model(&data);
    let elapsed = start.elapsed();
    let stats = model.stats();
    eprintln!(
        "  {} base variables, {} constraints (built in {:.2?})",
        stats.base_variable_count, stats.user_constraint_count, elapsed,
    );

    eprintln!("Solving...");
    let solver = collomatique_ilp::solvers::collo_cbc::ColloCbcSolver::with_disable_logging(false);
    match model.solve(&solver) {
        Some(solution) => {
            let config = solution.get_data();
            let assignments = collomatique_constraints_rooms::extract_assignments(&data, &config);
            for assignment in &assignments {
                let req = &data.requests[assignment.request];
                let room_str: &str = assignment.room.as_ref();
                if let Some(prep) = &assignment.prep_room {
                    let prep_str: &str = prep.as_ref();
                    println!(
                        "Request {}: {} {}h {} — Room: {}, Prep: {}",
                        assignment.request,
                        req.day,
                        *req.hour,
                        req.subject.as_ref() as &str,
                        room_str,
                        prep_str,
                    );
                } else {
                    println!(
                        "Request {}: {} {}h {} — Room: {}",
                        assignment.request,
                        req.day,
                        *req.hour,
                        req.subject.as_ref() as &str,
                        room_str,
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
        DemandConflictKind::InterrogationPrep => {
            eprintln!(
                "Warning: room \"{room}\" demanded for both interrogation \
                 and prep on {} at {}:",
                conflict.day, conflict.hour,
            );
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

fn print_demand_request(data: &ScheduleData, request: usize, kind: &str) {
    let req = &data.requests[request];
    eprintln!(
        "  - Request {request} ({kind}): {}, teacher: {}, requester: {}",
        req.subject.as_ref() as &str,
        req.teacher.as_ref() as &str,
        req.requester.as_ref() as &str,
    );
}

fn print_prep_demand_request(data: &ScheduleData, request: usize) {
    let req = &data.requests[request];
    eprintln!(
        "  - Request {request} ({} prep students): {}, teacher: {}, requester: {}",
        req.prep_students,
        req.subject.as_ref() as &str,
        req.teacher.as_ref() as &str,
        req.requester.as_ref() as &str,
    );
}

fn print_busiest_slot(data: &ScheduleData) {
    let mut slot_counts: BTreeMap<(collomatique_time::Weekday, Hour), Vec<usize>> = BTreeMap::new();
    for (i, req) in data.requests.iter().enumerate() {
        slot_counts.entry((req.day, req.hour)).or_default().push(i);
    }

    let (busiest_slot, busiest_reqs) = match slot_counts.iter().max_by_key(|(_, reqs)| {
        reqs.iter()
            .map(|&i| {
                let req = &data.requests[i];
                1u32 + if req.prep_students >= 1 { 1 } else { 0 }
            })
            .sum::<u32>()
    }) {
        Some(entry) => entry,
        None => return,
    };

    let total_rooms: u32 = busiest_reqs
        .iter()
        .map(|&i| {
            let req = &data.requests[i];
            1 + if req.prep_students >= 1 { 1 } else { 0 }
        })
        .sum();

    eprintln!(
        "Busiest slot: {} {}h — {} requests, {} room needs:",
        busiest_slot.0,
        *busiest_slot.1,
        busiest_reqs.len(),
        total_rooms,
    );
    for &req_idx in busiest_reqs {
        let req = &data.requests[req_idx];
        let pref_str = match &req.room_preference {
            Some(RoomPreference::Demand(r)) => format!(", demand={}", r.as_ref() as &str),
            Some(RoomPreference::Suggestion(r)) => format!(", suggest={}", r.as_ref() as &str),
            None => String::new(),
        };
        let prep_str = if req.prep_students >= 1 {
            let prep_pref = match &req.prep_preference {
                Some(RoomPreference::Demand(r)) => format!(" demand={}", r.as_ref() as &str),
                Some(RoomPreference::Suggestion(r)) => format!(" suggest={}", r.as_ref() as &str),
                None => String::new(),
            };
            format!(", prep={}{}", req.prep_students, prep_pref)
        } else {
            String::new()
        };
        eprintln!(
            "  [{req_idx}] {}: {} students, teacher={}, window={}{}{} [p1={} p2={} p3={}]",
            req.subject.as_ref() as &str,
            req.students.get(),
            req.teacher.as_ref() as &str,
            req.window,
            pref_str,
            prep_str,
            req.periods.p1,
            req.periods.p2,
            req.periods.p3,
        );
    }
}
