/// Hour of an interrogation, guaranteed to be between 8 and 19 inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hour(u32);

impl Hour {
    pub fn new(value: u32) -> Option<Hour> {
        (8..=19).contains(&value).then_some(Hour(value))
    }
}

impl std::ops::Deref for Hour {
    type Target = u32;

    fn deref(&self) -> &u32 {
        &self.0
    }
}
