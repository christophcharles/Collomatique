use collomatique_rooms_model::{InterrogationRoomPreference, PrepRoomPreference, ScheduleData};
use non_empty_string::NonEmptyString;

pub const PERIOD_COUNT: usize = 3;

pub struct VarEnv {
    pub data: ScheduleData,
    pub managed_rooms: Vec<NonEmptyString>,
}

impl VarEnv {
    pub fn new(data: &ScheduleData) -> Self {
        let managed_rooms = data
            .rooms
            .iter()
            .filter(|(_, r)| r.priority.is_some())
            .map(|(name, _)| name.clone())
            .collect();
        VarEnv {
            data: data.clone(),
            managed_rooms,
        }
    }

    pub(crate) fn has_interrogation_var(&self, request: usize, room: &NonEmptyString) -> bool {
        self.managed_rooms.contains(room)
            || self.data.requests[request]
                .room_preference
                .iter()
                .any(|p| {
                    matches!(p, InterrogationRoomPreference::Demand { room: name, .. } if name == room)
                })
    }

    pub(crate) fn has_prep_var(&self, request: usize, room: &NonEmptyString) -> bool {
        self.data.requests[request].prep_students >= 1
            && (self.managed_rooms.contains(room)
                || self.data.requests[request]
                    .prep_preference
                    .iter()
                    .any(|p| matches!(p, PrepRoomPreference::Demand(name) if name == room)))
    }
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, collomatique_ilp_modeler::DescribeVar,
)]
#[env(VarEnv)]
pub enum Var {
    RoomForInterrogation {
        #[range(Self::compute_all_request_range(env))]
        request: usize,
        #[range(Self::compute_interrogation_room_range(env, request))]
        room: NonEmptyString,
    },
    RoomForPrep {
        #[range(Self::compute_prep_request_range(env))]
        request: usize,
        #[range(Self::compute_prep_room_range(env, request))]
        room: NonEmptyString,
    },
}

impl Var {
    pub fn compute_all_request_range(env: &VarEnv) -> Vec<usize> {
        (0..env.data.requests.len()).collect()
    }

    pub fn compute_prep_request_range(env: &VarEnv) -> Vec<usize> {
        env.data
            .requests
            .iter()
            .enumerate()
            .filter(|(_, req)| req.prep_students >= 1)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn compute_interrogation_room_range(env: &VarEnv, request: &usize) -> Vec<NonEmptyString> {
        let mut rooms = env.managed_rooms.clone();
        for pref in &env.data.requests[*request].room_preference {
            if let InterrogationRoomPreference::Demand { room: name, .. } = pref {
                if !rooms.contains(name) {
                    rooms.push(name.clone());
                }
            }
        }
        rooms
    }

    pub fn compute_prep_room_range(env: &VarEnv, request: &usize) -> Vec<NonEmptyString> {
        let mut rooms = env.managed_rooms.clone();
        for pref in &env.data.requests[*request].prep_preference {
            if let PrepRoomPreference::Demand(name) = pref {
                if !rooms.contains(name) {
                    rooms.push(name.clone());
                }
            }
        }
        rooms
    }
}
