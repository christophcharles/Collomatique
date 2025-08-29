//! Common submodule
//!
//! This module contains common types that can be reused in
//! different entries.
//!
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonWithContact {
    pub firstname: String,
    pub surname: String,
    pub telephone: Option<non_empty_string::NonEmptyString>,
    pub email: Option<non_empty_string::NonEmptyString>,
}

impl From<&collomatique_state_colloscopes::PersonWithContact> for PersonWithContact {
    fn from(value: &collomatique_state_colloscopes::PersonWithContact) -> Self {
        PersonWithContact {
            firstname: value.firstname.clone(),
            surname: value.surname.clone(),
            telephone: value.tel.clone(),
            email: value.email.clone(),
        }
    }
}

impl From<PersonWithContact> for collomatique_state_colloscopes::PersonWithContact {
    fn from(value: PersonWithContact) -> Self {
        collomatique_state_colloscopes::PersonWithContact {
            firstname: value.firstname,
            surname: value.surname,
            tel: value.telephone,
            email: value.email,
        }
    }
}
