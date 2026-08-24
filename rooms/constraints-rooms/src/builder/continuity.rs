use std::collections::{BTreeSet, HashMap};

use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::Hour;
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

use super::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let mut groups: HashMap<(NonEmptyString, Weekday, Hour), Vec<usize>> = HashMap::new();

    for (req_idx, req) in env.data.requests.iter().enumerate() {
        if req.skip_room_continuity {
            continue;
        }
        let zone = env.data.config.time_zones.zone_label(req.hour);
        groups
            .entry((req.teacher.clone(), req.day, zone))
            .or_default()
            .push(req_idx);
    }

    for group in groups.values() {
        let mut hours_to_requests: HashMap<Hour, Vec<usize>> = HashMap::new();
        for &req_idx in group {
            hours_to_requests
                .entry(env.data.requests[req_idx].hour)
                .or_default()
                .push(req_idx);
        }

        let mut hours: Vec<Hour> = hours_to_requests.keys().copied().collect();
        hours.sort();

        for window in hours.windows(2) {
            let (h_curr, h_next) = (window[0], window[1]);
            if *h_next != *h_curr + 1 {
                continue;
            }

            let reqs_curr = &hours_to_requests[&h_curr];
            let reqs_next = &hours_to_requests[&h_next];

            for &r_a in reqs_curr {
                for &r_b in reqs_next {
                    if !env.data.requests[r_a]
                        .periods
                        .overlaps_with(&env.data.requests[r_b].periods)
                    {
                        continue;
                    }

                    let rooms_a: BTreeSet<NonEmptyString> =
                        Var::compute_interrogation_room_range(env, &r_a)
                            .into_iter()
                            .collect();
                    let rooms_b: BTreeSet<NonEmptyString> =
                        Var::compute_interrogation_room_range(env, &r_b)
                            .into_iter()
                            .collect();

                    for room in rooms_a.intersection(&rooms_b) {
                        let var_a = IntLinExpr::<V>::var(base_var(Var::RoomForInterrogation {
                            request: r_a,
                            room: room.clone(),
                        }));
                        let var_b = IntLinExpr::<V>::var(base_var(Var::RoomForInterrogation {
                            request: r_b,
                            room: room.clone(),
                        }));
                        bundle = bundle.with_constraint(
                            (var_a - var_b).eq(&IntLinExpr::constant(0)),
                            ConstraintDesc::RoomContinuityEqual {
                                request_a: r_a,
                                request_b: r_b,
                                room: room.clone(),
                            },
                        );
                    }

                    for room in rooms_a.difference(&rooms_b) {
                        let var_a = IntLinExpr::<V>::var(base_var(Var::RoomForInterrogation {
                            request: r_a,
                            room: room.clone(),
                        }));
                        bundle = bundle.with_constraint(
                            var_a.leq(&IntLinExpr::constant(0)),
                            ConstraintDesc::RoomContinuityExcluded {
                                request: r_a,
                                room: room.clone(),
                                neighbor_request: r_b,
                            },
                        );
                    }

                    for room in rooms_b.difference(&rooms_a) {
                        let var_b = IntLinExpr::<V>::var(base_var(Var::RoomForInterrogation {
                            request: r_b,
                            room: room.clone(),
                        }));
                        bundle = bundle.with_constraint(
                            var_b.leq(&IntLinExpr::constant(0)),
                            ConstraintDesc::RoomContinuityExcluded {
                                request: r_b,
                                room: room.clone(),
                                neighbor_request: r_a,
                            },
                        );
                    }
                }
            }
        }
    }

    bundle
}
