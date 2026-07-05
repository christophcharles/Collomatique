use std::collections::BTreeMap;

use collomatique_rooms_model::{
    InterrogationRoomStatus, PrepRoomStatus, ProximityType, Request, Room,
};
use non_empty_string::NonEmptyString;

const MAX_DISTANCE: f32 = 10.0;
const BLEED_FRACTION: f64 = 0.5;

fn combine(a: f64, b: f64) -> f64 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else {
        a.min(b)
    }
}

fn include_contribution(
    map: &mut BTreeMap<NonEmptyString, f64>,
    room: &NonEmptyString,
    value: f64,
) {
    let entry = map.entry(room.clone()).or_insert(0.0);
    *entry = combine(*entry, value);
}

pub(crate) fn compute_heat_map(
    request: &Request,
    rooms: &BTreeMap<NonEmptyString, Room>,
    is_prep: bool,
) -> BTreeMap<NonEmptyString, f64> {
    let mut map = BTreeMap::new();

    if is_prep {
        for (name, status) in &request.prep_statuses {
            match status {
                PrepRoomStatus::Accepted | PrepRoomStatus::Demanded => {
                    include_contribution(&mut map, name, 1.0);
                }
                PrepRoomStatus::Excluded => {}
            }
        }
    } else {
        for (name, status) in &request.room_statuses {
            match status {
                InterrogationRoomStatus::Accepted { .. }
                | InterrogationRoomStatus::Demanded { .. } => {
                    include_contribution(&mut map, name, 1.0);
                }
                InterrogationRoomStatus::Excluded => {}
            }
        }
    }

    let proximity = if is_prep {
        &request.prep_proximity
    } else {
        &request.proximity
    };

    for (prox_type, details) in proximity {
        let level = details.level as f64;
        match prox_type {
            ProximityType::Room(target_name) => {
                if details.fuzzy {
                    if let Some(target) = rooms.get(target_name) {
                        for (name, room) in rooms {
                            if room.floor != target.floor {
                                continue;
                            }
                            let dist = (room.x - target.x).abs() + (room.y - target.y).abs();
                            if dist >= MAX_DISTANCE {
                                continue;
                            }
                            let value = level * (1.0 - dist as f64 / MAX_DISTANCE as f64);
                            include_contribution(&mut map, name, value);
                        }
                    } else {
                        include_contribution(&mut map, target_name, level);
                    }
                } else {
                    include_contribution(&mut map, target_name, level);
                }
            }
            ProximityType::Floor(target_floor) => {
                for (name, room) in rooms {
                    let diff = (room.floor as i64 - *target_floor as i64).unsigned_abs();
                    let value = if diff == 0 {
                        level
                    } else if details.fuzzy && diff == 1 {
                        level * BLEED_FRACTION
                    } else {
                        continue;
                    };
                    include_contribution(&mut map, name, value);
                }
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_rooms_model::{BoardRequirement, Hour, Periods, ProximityDetails, Window};
    use collomatique_time::Weekday;
    use std::num::NonZeroU32;

    fn nes(s: &str) -> NonEmptyString {
        NonEmptyString::try_from(s.to_string()).unwrap()
    }

    fn test_request() -> Request {
        Request {
            periods: Periods {
                p1: false,
                p2: false,
                p3: true,
            },
            day: Weekday::from_french("Lundi").unwrap(),
            hour: Hour::new(8).unwrap(),
            subjects: vec![],
            classes: vec![],
            requester: nes("test"),
            teacher: nes("test"),
            boards: BoardRequirement::Zero,
            window: false,
            students: NonZeroU32::new(1).unwrap(),
            prep_students: 0,
            room_statuses: BTreeMap::new(),
            proximity: BTreeMap::new(),
            prep_statuses: BTreeMap::new(),
            prep_proximity: BTreeMap::new(),
            skip_room_continuity: false,
        }
    }

    fn test_room(floor: u32, x: f32, y: f32) -> Room {
        Room {
            floor,
            x,
            y,
            blackboards: 1.0,
            whiteboards: 0.0,
            capacity: NonZeroU32::new(30).unwrap(),
            window: Window::Exterior,
            priority: Some(0),
            reserved: false,
        }
    }

    fn rooms_map(entries: Vec<(&str, Room)>) -> BTreeMap<NonEmptyString, Room> {
        entries.into_iter().map(|(n, r)| (nes(n), r)).collect()
    }

    #[test]
    fn combine_both_positive_takes_max() {
        assert_eq!(combine(0.3, 0.8), 0.8);
        assert_eq!(combine(0.8, 0.3), 0.8);
    }

    #[test]
    fn combine_both_negative_takes_min() {
        assert_eq!(combine(-0.3, -0.8), -0.8);
        assert_eq!(combine(-0.8, -0.3), -0.8);
    }

    #[test]
    fn combine_mixed_takes_negative() {
        assert_eq!(combine(0.5, -0.3), -0.3);
        assert_eq!(combine(-0.3, 0.5), -0.3);
    }

    #[test]
    fn accepted_interrogation_gives_plus_one() {
        let mut req = test_request();
        req.room_statuses.insert(
            nes("A"),
            InterrogationRoomStatus::Accepted {
                can_share_with_prep: false,
            },
        );
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], 1.0);
    }

    #[test]
    fn demanded_interrogation_gives_plus_one() {
        let mut req = test_request();
        req.room_statuses.insert(
            nes("A"),
            InterrogationRoomStatus::Demanded {
                can_share_with_prep: false,
            },
        );
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], 1.0);
    }

    #[test]
    fn accepted_prep_gives_plus_one() {
        let mut req = test_request();
        req.prep_statuses.insert(nes("A"), PrepRoomStatus::Accepted);
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, true);
        assert_eq!(heat[&nes("A")], 1.0);
    }

    #[test]
    fn excluded_gives_no_contribution() {
        let mut req = test_request();
        req.room_statuses
            .insert(nes("A"), InterrogationRoomStatus::Excluded);
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert!(!heat.contains_key(&nes("A")));
    }

    #[test]
    fn exact_room_proximity() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Room(nes("A")),
            ProximityDetails {
                fuzzy: false,
                level: 0.5,
            },
        );
        let rooms = rooms_map(vec![
            ("A", test_room(0, 0.0, 0.0)),
            ("B", test_room(0, 1.0, 0.0)),
        ]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], 0.5);
        assert!(!heat.contains_key(&nes("B")));
    }

    #[test]
    fn fuzzy_room_proximity_decays_with_distance() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Room(nes("A")),
            ProximityDetails {
                fuzzy: true,
                level: 1.0,
            },
        );
        let rooms = rooms_map(vec![
            ("A", test_room(0, 0.0, 0.0)),
            ("B", test_room(0, 3.0, 0.0)),
            ("C", test_room(0, 10.0, 0.0)),
            ("D", test_room(1, 1.0, 0.0)),
        ]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], 1.0);
        assert!((heat[&nes("B")] - 0.7).abs() < 1e-9);
        assert!(!heat.contains_key(&nes("C")));
        assert!(!heat.contains_key(&nes("D")));
    }

    #[test]
    fn exact_floor_proximity() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Floor(1),
            ProximityDetails {
                fuzzy: false,
                level: 0.5,
            },
        );
        let rooms = rooms_map(vec![
            ("A", test_room(0, 0.0, 0.0)),
            ("B", test_room(1, 0.0, 0.0)),
            ("C", test_room(1, 5.0, 0.0)),
            ("D", test_room(2, 0.0, 0.0)),
        ]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert!(!heat.contains_key(&nes("A")));
        assert_eq!(heat[&nes("B")], 0.5);
        assert_eq!(heat[&nes("C")], 0.5);
        assert!(!heat.contains_key(&nes("D")));
    }

    #[test]
    fn fuzzy_floor_proximity_bleeds_to_adjacent() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Floor(1),
            ProximityDetails {
                fuzzy: true,
                level: 1.0,
            },
        );
        let rooms = rooms_map(vec![
            ("A", test_room(0, 0.0, 0.0)),
            ("B", test_room(1, 0.0, 0.0)),
            ("C", test_room(2, 0.0, 0.0)),
            ("D", test_room(3, 0.0, 0.0)),
        ]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], BLEED_FRACTION);
        assert_eq!(heat[&nes("B")], 1.0);
        assert_eq!(heat[&nes("C")], BLEED_FRACTION);
        assert!(!heat.contains_key(&nes("D")));
    }

    #[test]
    fn two_positive_sources_take_max() {
        let mut req = test_request();
        req.room_statuses.insert(
            nes("A"),
            InterrogationRoomStatus::Accepted {
                can_share_with_prep: false,
            },
        );
        req.proximity.insert(
            ProximityType::Room(nes("A")),
            ProximityDetails {
                fuzzy: false,
                level: 0.5,
            },
        );
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], 1.0);
    }

    #[test]
    fn negative_proximity_overrides_accepted() {
        let mut req = test_request();
        req.room_statuses.insert(
            nes("A"),
            InterrogationRoomStatus::Accepted {
                can_share_with_prep: false,
            },
        );
        req.proximity.insert(
            ProximityType::Room(nes("A")),
            ProximityDetails {
                fuzzy: false,
                level: -0.5,
            },
        );
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("A")], -0.5);
    }

    #[test]
    fn prep_uses_prep_proximity() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Room(nes("A")),
            ProximityDetails {
                fuzzy: false,
                level: 0.5,
            },
        );
        req.prep_proximity.insert(
            ProximityType::Room(nes("B")),
            ProximityDetails {
                fuzzy: false,
                level: 0.25,
            },
        );
        let rooms = rooms_map(vec![
            ("A", test_room(0, 0.0, 0.0)),
            ("B", test_room(0, 1.0, 0.0)),
        ]);
        let heat = compute_heat_map(&req, &rooms, true);
        assert!(!heat.contains_key(&nes("A")));
        assert_eq!(heat[&nes("B")], 0.25);
    }

    #[test]
    fn fuzzy_room_unknown_target_falls_back_to_exact() {
        let mut req = test_request();
        req.proximity.insert(
            ProximityType::Room(nes("UNKNOWN")),
            ProximityDetails {
                fuzzy: true,
                level: 0.75,
            },
        );
        let rooms = rooms_map(vec![("A", test_room(0, 0.0, 0.0))]);
        let heat = compute_heat_map(&req, &rooms, false);
        assert_eq!(heat[&nes("UNKNOWN")], 0.75);
        assert!(!heat.contains_key(&nes("A")));
    }
}
