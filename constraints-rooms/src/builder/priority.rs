use std::collections::{BTreeMap, BTreeSet};

use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::{Hour, Periods, Request, RoomPreference};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

use super::{MyBundle, V, base_var, extra_var};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{PERIOD_COUNT, Var, VarEnv};

fn period_active(periods: &Periods, period: usize) -> bool {
    match period {
        0 => periods.p1,
        1 => periods.p2,
        2 => periods.p3,
        _ => false,
    }
}

fn request_active_at(req: &Request, period: usize, day: &Weekday, hour: &Hour) -> bool {
    period_active(&req.periods, period) && req.day == *day && req.hour == *hour
}

fn is_room_blocked(
    env: &VarEnv,
    room_name: &NonEmptyString,
    period: usize,
    day: &Weekday,
    hour: &Hour,
) -> bool {
    if let Some(room) = env.data.rooms.get(room_name) {
        if room.reserved && period_active(&env.data.config.oral_exam_periods, period) {
            return true;
        }
    }

    for incompat in &env.data.incompats {
        if incompat.room == *room_name
            && incompat.day == *day
            && incompat.hour == *hour
            && period_active(&incompat.periods, period)
        {
            return true;
        }
    }

    false
}

fn is_interrogation_demand(req: &Request, room_name: &NonEmptyString) -> bool {
    matches!(&req.room_preference, Some(RoomPreference::Demand(name)) if name == room_name)
}

fn is_prep_demand(req: &Request, room_name: &NonEmptyString) -> bool {
    matches!(&req.prep_preference, Some(RoomPreference::Demand(name)) if name == room_name)
}

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let priority_set: BTreeSet<u32> = env
        .data
        .rooms
        .iter()
        .filter_map(|(_, room)| room.priority)
        .collect();

    let priorities: Vec<u32> = priority_set.into_iter().collect();

    let time_slots: BTreeSet<(Weekday, Hour)> = env
        .data
        .requests
        .iter()
        .map(|req| (req.day, req.hour))
        .collect();

    let rooms_by_priority: BTreeMap<u32, Vec<NonEmptyString>> = {
        let mut map: BTreeMap<u32, Vec<NonEmptyString>> = BTreeMap::new();
        for (name, room) in &env.data.rooms {
            if let Some(p) = room.priority {
                map.entry(p).or_default().push(name.clone());
            }
        }
        map
    };

    let max_priority = env.data.config.max_priority;

    if let Some(max_p) = max_priority {
        for (&prio, rooms) in &rooms_by_priority {
            if prio <= max_p {
                continue;
            }
            for room_name in rooms {
                for (req_idx, req) in env.data.requests.iter().enumerate() {
                    if env.has_interrogation_var(req_idx, room_name)
                        && !is_interrogation_demand(req, room_name)
                    {
                        bundle = bundle.with_constraint(
                            IntLinExpr::var(base_var(Var::RoomForInterrogation {
                                request: req_idx,
                                room: room_name.clone(),
                            }))
                            .leq(&IntLinExpr::constant(0)),
                            ConstraintDesc::MaxPriorityInterrogation {
                                request: req_idx,
                                room: room_name.clone(),
                            },
                        );
                    }
                    if env.has_prep_var(req_idx, room_name) && !is_prep_demand(req, room_name) {
                        bundle = bundle.with_constraint(
                            IntLinExpr::var(base_var(Var::RoomForPrep {
                                request: req_idx,
                                room: room_name.clone(),
                            }))
                            .leq(&IntLinExpr::constant(0)),
                            ConstraintDesc::MaxPriorityPrep {
                                request: req_idx,
                                room: room_name.clone(),
                            },
                        );
                    }
                }
            }
        }
    }

    for period in 0..PERIOD_COUNT {
        for &(day, hour) in &time_slots {
            let active_requests: Vec<usize> = env
                .data
                .requests
                .iter()
                .enumerate()
                .filter(|(_, req)| request_active_at(req, period, &day, &hour))
                .map(|(i, _)| i)
                .collect();

            if active_requests.is_empty() {
                continue;
            }

            for (p_idx, &priority) in priorities.iter().enumerate() {
                if let Some(max_p) = max_priority {
                    if priority > max_p {
                        continue;
                    }
                }

                let available_rooms: Vec<&NonEmptyString> = rooms_by_priority
                    .get(&priority)
                    .into_iter()
                    .flatten()
                    .filter(|room_name| !is_room_blocked(env, room_name, period, &day, &hour))
                    .collect();

                let mut reify_constraints = Vec::new();

                for room_name in &available_rooms {
                    let sum: IntLinExpr<V> = active_requests
                        .iter()
                        .filter_map(|&req_idx| {
                            let mut expr = IntLinExpr::constant(0);
                            let mut has_any = false;

                            if env.has_interrogation_var(req_idx, room_name) {
                                expr = expr
                                    + IntLinExpr::var(base_var(Var::RoomForInterrogation {
                                        request: req_idx,
                                        room: (*room_name).clone(),
                                    }));
                                has_any = true;
                            }

                            if env.has_prep_var(req_idx, room_name) {
                                expr = expr
                                    + IntLinExpr::var(base_var(Var::RoomForPrep {
                                        request: req_idx,
                                        room: (*room_name).clone(),
                                    }));
                                has_any = true;
                            }

                            has_any.then_some(expr)
                        })
                        .sum();

                    reify_constraints.push(sum.geq(&IntLinExpr::constant(1)));
                }

                if p_idx > 0 {
                    reify_constraints.push(
                        IntLinExpr::var(extra_var(ExtraVarName::PriorityExhausted {
                            period,
                            day,
                            hour,
                            priority: priorities[p_idx - 1],
                        }))
                        .geq(&IntLinExpr::constant(1)),
                    );
                }

                if reify_constraints.is_empty() {
                    reify_constraints.push(IntLinExpr::constant(0).leq(&IntLinExpr::constant(0)));
                }

                bundle = bundle
                    .and_reified(
                        ExtraVarName::PriorityExhausted {
                            period,
                            day,
                            hour,
                            priority,
                        },
                        move || reify_constraints,
                    )
                    .expect("no duplicate extras");
            }

            for (p_idx, &current_priority) in priorities.iter().enumerate() {
                if p_idx == 0 {
                    continue;
                }
                if let Some(max_p) = max_priority {
                    if current_priority > max_p {
                        continue;
                    }
                }

                let prev_priority = priorities[p_idx - 1];

                let current_rooms = match rooms_by_priority.get(&current_priority) {
                    Some(rooms) => rooms,
                    None => continue,
                };

                let exhausted_var = extra_var(ExtraVarName::PriorityExhausted {
                    period,
                    day,
                    hour,
                    priority: prev_priority,
                });

                for room_name in current_rooms {
                    for &req_idx in &active_requests {
                        let req = &env.data.requests[req_idx];

                        if env.has_interrogation_var(req_idx, room_name)
                            && !is_interrogation_demand(req, room_name)
                        {
                            bundle = bundle.with_constraint(
                                IntLinExpr::var(base_var(Var::RoomForInterrogation {
                                    request: req_idx,
                                    room: room_name.clone(),
                                }))
                                .leq(&IntLinExpr::var(exhausted_var.clone())),
                                ConstraintDesc::PriorityInterrogation {
                                    request: req_idx,
                                    room: room_name.clone(),
                                    period,
                                    day,
                                    hour,
                                },
                            );
                        }

                        if env.has_prep_var(req_idx, room_name) && !is_prep_demand(req, room_name) {
                            bundle = bundle.with_constraint(
                                IntLinExpr::var(base_var(Var::RoomForPrep {
                                    request: req_idx,
                                    room: room_name.clone(),
                                }))
                                .leq(&IntLinExpr::var(exhausted_var.clone())),
                                ConstraintDesc::PriorityPrep {
                                    request: req_idx,
                                    room: room_name.clone(),
                                    period,
                                    day,
                                    hour,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    bundle
}
