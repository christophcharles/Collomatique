use std::collections::{BTreeSet, HashSet};

use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::{Hour, Request};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

use crate::builder::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{PERIOD_COUNT, Var, VarEnv};

fn request_active_at(req: &Request, period: usize, day: &Weekday, hour: &Hour) -> bool {
    let in_period = match period {
        0 => req.p1,
        1 => req.p2,
        2 => req.p3,
        _ => false,
    };
    in_period && req.day == *day && req.hour == *hour
}

fn has_interrogation_var(env: &VarEnv, request: usize, room: &NonEmptyString) -> bool {
    env.managed_rooms.contains(room)
        || env.data.requests[request]
            .room_suggestion
            .as_ref()
            .is_some_and(|s| s == room)
}

fn has_prep_var(env: &VarEnv, request: usize, room: &NonEmptyString) -> bool {
    env.data.requests[request].prep_students >= 1
        && (env.managed_rooms.contains(room)
            || env.data.requests[request]
                .prep_suggestion
                .as_ref()
                .is_some_and(|s| s == room))
}

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let time_slots: BTreeSet<(Weekday, Hour)> = env
        .data
        .requests
        .iter()
        .map(|req| (req.day, req.hour))
        .collect();

    let declared_room_names: HashSet<&str> = env
        .data
        .rooms
        .iter()
        .map(|r| r.name.as_ref() as &str)
        .collect();

    // Declared rooms (managed or not)
    for room in &env.data.rooms {
        let capacity = room.capacity.get();
        let is_managed = room.priority.is_some();

        for period in 0..PERIOD_COUNT {
            for &(day, hour) in &time_slots {
                let relevant: Vec<usize> = env
                    .data
                    .requests
                    .iter()
                    .enumerate()
                    .filter(|(i, req)| {
                        if !request_active_at(req, period, &day, &hour) {
                            return false;
                        }
                        if is_managed {
                            true
                        } else {
                            has_interrogation_var(env, *i, &room.name)
                                || has_prep_var(env, *i, &room.name)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();

                if relevant.is_empty() {
                    continue;
                }

                let sum: IntLinExpr<V> = relevant
                    .iter()
                    .map(|&req| {
                        let req_data = &env.data.requests[req];
                        let mut expr = IntLinExpr::constant(0);

                        if has_interrogation_var(env, req, &room.name) {
                            let coeff = std::cmp::max(capacity, req_data.students.get()) as i64;
                            expr = expr
                                + coeff
                                    * IntLinExpr::var(base_var(Var::RoomForInterrogation {
                                        request: req,
                                        room: room.name.clone(),
                                    }));
                        }

                        if has_prep_var(env, req, &room.name) {
                            expr = expr
                                + req_data.prep_students as i64
                                    * IntLinExpr::var(base_var(Var::RoomForPrep {
                                        request: req,
                                        room: room.name.clone(),
                                    }));
                        }

                        expr
                    })
                    .sum();

                bundle = bundle.with_constraint(
                    sum.leq(&IntLinExpr::constant(capacity as i64)),
                    ConstraintDesc::RoomNotOverused {
                        room: room.name.clone(),
                        period,
                        day,
                        hour,
                    },
                );
            }
        }
    }

    // Undeclared rooms (in suggestions but not in rooms.csv)
    let mut undeclared_rooms: BTreeSet<NonEmptyString> = BTreeSet::new();
    for req in &env.data.requests {
        if let Some(sug) = &req.room_suggestion {
            if !declared_room_names.contains(sug.as_ref() as &str) {
                undeclared_rooms.insert(sug.clone());
            }
        }
        if let Some(sug) = &req.prep_suggestion {
            if !declared_room_names.contains(sug.as_ref() as &str) {
                undeclared_rooms.insert(sug.clone());
            }
        }
    }

    for room in &undeclared_rooms {
        for period in 0..PERIOD_COUNT {
            for &(day, hour) in &time_slots {
                let relevant: Vec<usize> = env
                    .data
                    .requests
                    .iter()
                    .enumerate()
                    .filter(|(i, req)| {
                        request_active_at(req, period, &day, &hour)
                            && (has_interrogation_var(env, *i, room) || has_prep_var(env, *i, room))
                    })
                    .map(|(i, _)| i)
                    .collect();

                if relevant.is_empty() {
                    continue;
                }

                let sum: IntLinExpr<V> = relevant
                    .iter()
                    .map(|&req| {
                        let mut expr = IntLinExpr::constant(0);

                        if has_interrogation_var(env, req, room) {
                            expr = expr
                                + IntLinExpr::var(base_var(Var::RoomForInterrogation {
                                    request: req,
                                    room: room.clone(),
                                }));
                        }

                        if has_prep_var(env, req, room) {
                            expr = expr
                                + IntLinExpr::var(base_var(Var::RoomForPrep {
                                    request: req,
                                    room: room.clone(),
                                }));
                        }

                        expr
                    })
                    .sum();

                bundle = bundle.with_constraint(
                    sum.leq(&IntLinExpr::constant(1)),
                    ConstraintDesc::UndeclaredRoomExclusive {
                        room: room.clone(),
                        period,
                        day,
                        hour,
                    },
                );
            }
        }
    }

    bundle
}
