use collomatique_rooms_model::ScheduleData;
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
        if let Some(suggestion) = env.data.requests[*request].room_suggestion.as_ref() {
            if !rooms.contains(suggestion) {
                rooms.push(suggestion.clone());
            }
        }
        rooms
    }

    pub fn compute_prep_room_range(env: &VarEnv, request: &usize) -> Vec<NonEmptyString> {
        let mut rooms = env.managed_rooms.clone();
        if let Some(suggestion) = env.data.requests[*request].prep_suggestion.as_ref() {
            if !rooms.contains(suggestion) {
                rooms.push(suggestion.clone());
            }
        }
        rooms
    }
}
