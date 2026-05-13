use std::collections::BTreeSet;

use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_rooms_model::{Hour, InterrogationRoomPreference, PrepRoomPreference, Request};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

use super::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{PERIOD_COUNT, Var, VarEnv};

fn request_active_at(req: &Request, period: usize, day: &Weekday, hour: &Hour) -> bool {
    let in_period = match period {
        0 => req.periods.p1,
        1 => req.periods.p2,
        2 => req.periods.p3,
        _ => false,
    };
    in_period && req.day == *day && req.hour == *hour
}

pub(crate) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();

    let time_slots: BTreeSet<(Weekday, Hour)> = env
        .data
        .requests
        .iter()
        .map(|req| (req.day, req.hour))
        .collect();

    let mut all_rooms: BTreeSet<NonEmptyString> = env.data.rooms.keys().cloned().collect();

    for req in &env.data.requests {
        if let Some(InterrogationRoomPreference::Demand { room, .. }) = &req.room_preference {
            all_rooms.insert(room.clone());
        }
        if let Some(PrepRoomPreference::Demand(name)) = &req.prep_preference {
            all_rooms.insert(name.clone());
        }
    }

    for room_name in &all_rooms {
        for period in 0..PERIOD_COUNT {
            for &(day, hour) in &time_slots {
                let relevant: Vec<usize> = env
                    .data
                    .requests
                    .iter()
                    .enumerate()
                    .filter(|(i, req)| {
                        request_active_at(req, period, &day, &hour)
                            && env.has_interrogation_var(*i, room_name)
                    })
                    .map(|(i, _)| i)
                    .collect();

                if relevant.is_empty() {
                    continue;
                }

                let sum: IntLinExpr<V> = relevant
                    .iter()
                    .map(|&req| {
                        IntLinExpr::var(base_var(Var::RoomForInterrogation {
                            request: req,
                            room: room_name.clone(),
                        }))
                    })
                    .sum();

                bundle = bundle.with_constraint(
                    sum.leq(&IntLinExpr::constant(1)),
                    ConstraintDesc::OneInterrogationPerRoom {
                        room: room_name.clone(),
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
