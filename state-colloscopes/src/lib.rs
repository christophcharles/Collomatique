//! Colloscopes state crate
//!
//! This crate implements the various concepts of [collomatique-state]
//! and the various traits for the specific case of colloscope representation.
//!

use collomatique_state::{tools, InMemoryData, Operation};
use std::collections::BTreeMap;

pub mod ids;
use ids::IdIssuer;
pub use ids::StudentId;
pub mod ops;
use ops::AnnotatedStudentOp;
pub use ops::{AnnotatedOp, Op, StudentOp};

/// Description of a person with contacts
///
/// This type is used to describe both students and teachers.
/// Each student and teacher has its own card with name and contacts.
/// There are not used for the colloscope solving process
/// but can help produce a nice colloscope output with contact info.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonWithContact {
    /// Surname of the person
    ///
    /// Though this field can be an empty string,
    /// it is considered mandatory internally
    pub surname: String,

    /// Firstname of the person
    ///
    /// Though this field can be an empty string,
    /// it is considered mandatory internally
    pub firstname: String,

    /// Person's telephone number
    ///
    /// This field is optional: this reflects the
    /// fact that some persons might not want to share
    /// their personal info or only some of it.
    pub tel: Option<String>,

    /// Person's email
    ///
    /// This field is optional: this reflects the
    /// fact that some persons might not want to share
    /// their personal info or only some of it.
    pub email: Option<String>,
}

/// Internal structure to store the data for [Data]
///
/// We have `data1 == data2` if and only if their internal
/// data is the same. This means they would lead to the same
/// file on disk. But the internal id issuer might have a different
/// state.
///
/// [InnerData] represents this actual 'on-disk' data so we can
/// directly use `derive(PartialEq, Eq)` with it. The implementation
/// of [Eq] and [PartialEq] for [Data] relies on it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InnerData {
    student_list: BTreeMap<StudentId, PersonWithContact>,
}

/// Complete data that can be handled in the colloscope
///
/// This [Data] structure contains all the data that can
/// be manipulated in collomatique. It contains the list
/// of students, of teachers, the various interrogations,
/// a description of constraints etc. It also contains the
/// various colloscopes that have been generated or edited.
///
/// It cannot be modified or accessed directly. To the other
/// crates, this is an opaque type.
///
/// It does not necesserally correlate exactly to the data stored
/// on disk. This is to allow versioning.
#[derive(Debug, Clone)]
pub struct Data {
    id_issuer: IdIssuer,
    inner_data: InnerData,
}

impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.inner_data == other.inner_data
    }
}

impl Eq for Data {}

use thiserror::Error;

/// Errors for colloscopes modification
///
/// These errors can be returned when trying to modifying [Data].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),
}

impl InMemoryData for Data {
    type OriginalOperation = Op;
    type AnnotatedOperation = AnnotatedOp;
    type Error = Error;

    fn annotate(&mut self, op: Op) -> AnnotatedOp {
        AnnotatedOp::annotate(op, &mut self.id_issuer)
    }

    fn build_rev_with_current_state(
        &self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => {
                Ok(AnnotatedOp::Student(self.build_rev_student(student_op)?))
            }
        }
    }

    fn apply(&mut self, op: &Self::AnnotatedOperation) -> std::result::Result<(), Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => self.apply_student(student_op),
        }
    }
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        let student_list = BTreeMap::new();
        Self::from_lists(student_list).expect("Lists are empty and should be valid")
    }

    /// Create a new [Data] from existing lists
    ///
    /// This will check the consistency of the lists
    /// and will also do some internal checks, so this might fail.
    pub fn from_lists(
        student_list: BTreeMap<u64, PersonWithContact>,
    ) -> Result<Data, tools::IdError> {
        let student_ids = student_list.keys().copied();
        let id_issuer = IdIssuer::new(student_ids)?;

        // Ids have been validated
        let student_list = unsafe {
            student_list
                .into_iter()
                .map(|(key, value)| (StudentId::new(key), value))
                .collect()
        };

        Ok(Data {
            id_issuer,
            inner_data: InnerData { student_list },
        })
    }

    /// Get the student list
    pub fn get_student_list(&self) -> &BTreeMap<StudentId, PersonWithContact> {
        &self.inner_data.student_list
    }

    /// Used internally
    ///
    /// Apply student operations
    fn apply_student(&mut self, student_op: &AnnotatedStudentOp) -> std::result::Result<(), Error> {
        match student_op {
            AnnotatedStudentOp::Add(student_id, student) => {
                if self.inner_data.student_list.contains_key(student_id) {
                    return Err(Error::StudentIdAlreadyExists(student_id.clone()));
                }

                self.inner_data
                    .student_list
                    .insert(student_id.clone(), student.clone());
                Ok(())
            }
            AnnotatedStudentOp::Remove(student_id) => {
                if self.inner_data.student_list.remove(&student_id).is_none() {
                    return Err(Error::InvalidStudentId(student_id.clone()));
                }
                Ok(())
            }
            AnnotatedStudentOp::Update(student_id, student) => {
                let Some(old_student) = self.inner_data.student_list.get_mut(&student_id) else {
                    return Err(Error::InvalidStudentId(student_id.clone()));
                };

                *old_student = student.clone();
                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a student operation
    fn build_rev_student(
        &self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<AnnotatedStudentOp, Error> {
        match student_op {
            AnnotatedStudentOp::Add(student_id, _student) => {
                if self.inner_data.student_list.contains_key(student_id) {
                    return Err(Error::StudentIdAlreadyExists(student_id.clone()));
                }

                Ok(AnnotatedStudentOp::Remove(student_id.clone()))
            }
            AnnotatedStudentOp::Remove(student_id) => {
                let Some(old_student) = self.inner_data.student_list.get(&student_id).cloned()
                else {
                    return Err(Error::InvalidStudentId(student_id.clone()));
                };

                Ok(AnnotatedStudentOp::Add(student_id.clone(), old_student))
            }
            AnnotatedStudentOp::Update(student_id, _student) => {
                let Some(old_student) = self.inner_data.student_list.get(&student_id).cloned()
                else {
                    return Err(Error::InvalidStudentId(student_id.clone()));
                };

                Ok(AnnotatedStudentOp::Update(student_id.clone(), old_student))
            }
        }
    }
}
