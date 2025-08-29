use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonWithContactMsg {
    pub firstname: String,
    pub surname: String,
    pub tel: Option<non_empty_string::NonEmptyString>,
    pub email: Option<non_empty_string::NonEmptyString>,
}

impl From<PersonWithContactMsg> for collomatique_state_colloscopes::PersonWithContact {
    fn from(value: PersonWithContactMsg) -> Self {
        collomatique_state_colloscopes::PersonWithContact {
            surname: value.surname,
            firstname: value.firstname,
            tel: value.tel,
            email: value.email,
        }
    }
}
