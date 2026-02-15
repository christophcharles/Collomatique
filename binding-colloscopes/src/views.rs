use collomatique_state_colloscopes::colloscope_params::Parameters;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub(crate) params: Parameters,
}

impl Env {
    pub fn get_params(&self) -> &Parameters {
        &self.params
    }
}

impl From<Parameters> for Env {
    fn from(value: Parameters) -> Self {
        Env { params: value }
    }
}
