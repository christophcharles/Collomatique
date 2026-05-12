pub mod parsing;

use std::path::Path;

pub use collomatique_rooms_model::{Hour, Request, Room, ScheduleData, Window};
pub use parsing::ScheduleError;

pub fn run(rooms: &Path, requests: &Path) -> Result<(), ScheduleError> {
    let data = parsing::parse_schedule(rooms, requests)?;
    eprintln!(
        "Parsed {} rooms and {} requests",
        data.rooms.len(),
        data.requests.len(),
    );
    for name in data.unregistered_rooms() {
        eprintln!(
            "Warning: room \"{name}\" is not registered in the rooms file. \
             In case of double occupancy, we will not be able to find the closest available room."
        );
    }

    eprintln!("Building ILP model...");
    let model = collomatique_constraints_rooms::build_model(&data);
    let stats = model.stats();
    eprintln!(
        "  {} base variables, {} constraints",
        stats.base_variable_count, stats.user_constraint_count,
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
