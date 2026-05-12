#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ExtraVarName {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ConstraintDesc {
    OneRoomPerRequest { request: usize },
    OnePrepRoomPerRequest { request: usize },
}
