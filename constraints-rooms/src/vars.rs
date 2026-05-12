use non_empty_string::NonEmptyString;

#[derive(Clone)]
pub struct RequestInput {
    pub needs_prep: bool,
    pub room_suggestion: Option<NonEmptyString>,
    pub prep_suggestion: Option<NonEmptyString>,
}

#[derive(Clone)]
pub struct RoomScheduleInput {
    pub managed_rooms: Vec<NonEmptyString>,
    pub requests: Vec<RequestInput>,
}

pub struct VarEnv {
    pub input: RoomScheduleInput,
}

impl VarEnv {
    pub fn new(input: RoomScheduleInput) -> Self {
        VarEnv { input }
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
        (0..env.input.requests.len()).collect()
    }

    pub fn compute_prep_request_range(env: &VarEnv) -> Vec<usize> {
        env.input
            .requests
            .iter()
            .enumerate()
            .filter(|(_, req)| req.needs_prep)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn compute_interrogation_room_range(env: &VarEnv, request: &usize) -> Vec<NonEmptyString> {
        let mut rooms = env.input.managed_rooms.clone();
        if let Some(suggestion) = env.input.requests[*request].room_suggestion.as_ref() {
            if !rooms.contains(suggestion) {
                rooms.push(suggestion.clone());
            }
        }
        rooms
    }

    pub fn compute_prep_room_range(env: &VarEnv, request: &usize) -> Vec<NonEmptyString> {
        let mut rooms = env.input.managed_rooms.clone();
        if let Some(suggestion) = env.input.requests[*request].prep_suggestion.as_ref() {
            if !rooms.contains(suggestion) {
                rooms.push(suggestion.clone());
            }
        }
        rooms
    }
}
