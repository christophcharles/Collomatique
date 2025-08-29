//! Colloscopes state crate
//!
//! This crate implements the various concepts of [collomatique-state]
//! and the various traits for the specific case of colloscope representation.
//!

use collomatique_state::{tools, InMemoryData, Operation};
use periods::{Periods, PeriodsExternalData};
use std::collections::{BTreeMap, BTreeSet};
use subjects::{Subjects, SubjectsExternalData};

pub mod ids;
use ids::IdIssuer;
pub use ids::{PeriodId, StudentId, SubjectId};
pub mod ops;
pub use ops::{AnnotatedOp, Op, PeriodOp, StudentOp};
use ops::{AnnotatedPeriodOp, AnnotatedStudentOp};

pub mod periods;
pub mod subjects;

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
    pub tel: Option<non_empty_string::NonEmptyString>,

    /// Person's email
    ///
    /// This field is optional: this reflects the
    /// fact that some persons might not want to share
    /// their personal info or only some of it.
    pub email: Option<non_empty_string::NonEmptyString>,
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
    periods: periods::Periods,
    subjects: subjects::Subjects,
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

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),
}

/// Potential new id returned by annotation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewId {
    StudentId(StudentId),
    PeriodId(PeriodId),
}

impl From<StudentId> for NewId {
    fn from(value: StudentId) -> Self {
        NewId::StudentId(value)
    }
}

impl From<PeriodId> for NewId {
    fn from(value: PeriodId) -> Self {
        NewId::PeriodId(value)
    }
}

impl InMemoryData for Data {
    type OriginalOperation = Op;
    type AnnotatedOperation = AnnotatedOp;
    type NewInfo = Option<NewId>;
    type Error = Error;

    fn annotate(&mut self, op: Op) -> (AnnotatedOp, Option<NewId>) {
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
            AnnotatedOp::Period(period_op) => {
                Ok(AnnotatedOp::Period(self.build_rev_period(period_op)?))
            }
        }
    }

    fn apply(&mut self, op: &Self::AnnotatedOperation) -> std::result::Result<(), Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => self.apply_student(student_op)?,
            AnnotatedOp::Period(period_op) => self.apply_period(period_op)?,
        }
        assert!(self.check_invariants());
        Ok(())
    }
}

impl Data {
    /// Promotes an u64 to a [PeriodId] if it is valid
    pub fn validate_period_id(&self, id: u64) -> Option<PeriodId> {
        for (period_id, _) in &self.inner_data.periods.ordered_period_list {
            if period_id.inner() == id {
                return Some(*period_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [StudentId] if it is valid
    pub fn validate_student_id(&self, id: u64) -> Option<StudentId> {
        let student_id = unsafe { StudentId::new(id) };

        if !self.inner_data.student_list.contains_key(&student_id) {
            return None;
        }

        Some(student_id)
    }
}

impl Data {
    /// USED INTERNALLY
    ///
    /// Checks that there are no duplicate ids in data
    ///
    /// Even ids for different type of data should be different
    fn check_no_duplicate_ids(&self) -> bool {
        let mut ids_so_far = BTreeSet::new();

        for (id, _) in &self.inner_data.periods.ordered_period_list {
            if !ids_so_far.insert(id.inner()) {
                return false;
            }
        }

        for (id, _) in &self.inner_data.subjects.ordered_period_list {
            if !ids_so_far.insert(id.inner()) {
                return false;
            }
        }

        for (id, _) in &self.inner_data.student_list {
            if !ids_so_far.insert(id.inner()) {
                return false;
            }
        }

        true
    }

    /// USED INTERNALLY
    ///
    /// Checks that all the periods ids used in subjects data are valid
    fn check_subjects_data_has_correct_period_ids(&self, period_ids: &BTreeSet<PeriodId>) -> bool {
        for (_subject_id, subject) in &self.inner_data.subjects.ordered_period_list {
            for period_id in &subject.excluded_periods {
                if !period_ids.contains(period_id) {
                    return false;
                }
            }
        }
        true
    }

    /// USED INTERNALLY
    ///
    /// Checks the various ranges in subjects
    /// In particular, students per group and groups per interrogation should
    ///
    fn check_subjects_data_has_correct_ranges(&self) -> bool {
        for (_subject_id, subject) in &self.inner_data.subjects.ordered_period_list {
            if subject.parameters.students_per_group.is_empty() {
                return false;
            }
            if subject.parameters.groups_per_interrogation.is_empty() {
                return false;
            }
        }
        true
    }

    /// USED INTERNALLY
    ///
    /// checks all that subjects have valid week numbers when used
    fn check_subjects_have_valid_week_numbers(&self, week_count: usize) -> bool {
        for (_subject_id, subject) in &self.inner_data.subjects.ordered_period_list {
            if let subjects::SubjectPeriodicity::OnceForEveryArbitraryBlock {
                weeks_at_start_of_new_block,
            } = &subject.parameters.periodicity
            {
                for week in weeks_at_start_of_new_block {
                    if *week >= week_count {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_subjects_data_consistency(
        &self,
        period_ids: &BTreeSet<PeriodId>,
        week_count: usize,
    ) -> bool {
        if !self.check_subjects_data_has_correct_period_ids(period_ids) {
            return false;
        }
        if !self.check_subjects_data_has_correct_ranges() {
            return false;
        }
        if !self.check_subjects_have_valid_week_numbers(week_count) {
            return false;
        }
        true
    }

    /// USED INTERNALLY
    ///
    /// Build the set of PeriodIds
    ///
    /// This is useful to check that references are valid
    fn build_period_ids(&self) -> BTreeSet<PeriodId> {
        let mut ids = BTreeSet::new();
        for (id, _) in &self.inner_data.periods.ordered_period_list {
            ids.insert(*id);
        }
        ids
    }

    /// USED INTERNALLY
    ///
    /// Compute the total number of weeks covered in periods
    ///
    /// This is useful to check that week numbers are valid
    fn build_week_count(&self) -> usize {
        self.inner_data
            .periods
            .ordered_period_list
            .iter()
            .fold(0usize, |acc, (_id, desc)| acc + desc.len())
    }

    /// USED INTERNALLY
    ///
    /// Checks all the invariants of data
    fn check_invariants(&self) -> bool {
        if !self.check_no_duplicate_ids() {
            return false;
        }
        let period_ids = self.build_period_ids();
        let week_count = self.build_week_count();
        if !self.check_subjects_data_consistency(&period_ids, week_count) {
            return false;
        }
        true
    }
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        let student_list = BTreeMap::new();
        Self::from_data(
            student_list,
            PeriodsExternalData::default(),
            SubjectsExternalData::default(),
        )
        .expect("Default data should be valid")
    }

    /// Create a new [Data] from existing data
    ///
    /// This will check the consistency of the data
    /// and will also do some internal checks, so this might fail.
    pub fn from_data(
        student_list: BTreeMap<u64, PersonWithContact>,
        periods: periods::PeriodsExternalData,
        subjects: subjects::SubjectsExternalData,
    ) -> Result<Data, tools::IdError> {
        let student_ids = student_list.keys().copied();
        let period_ids = periods.ordered_period_list.iter().map(|(id, _d)| *id);
        let subject_ids = subjects.ordered_period_list.iter().map(|(id, _d)| *id);
        let id_issuer = IdIssuer::new(student_ids, period_ids, subject_ids)?;

        let period_ids: std::collections::BTreeSet<_> = periods
            .ordered_period_list
            .iter()
            .map(|(id, _d)| *id)
            .collect();
        let week_count = periods
            .ordered_period_list
            .iter()
            .fold(0usize, |acc, (_id, desc)| acc + desc.len());
        if !subjects.validate_all(&period_ids, week_count) {
            return Err(tools::IdError::InvalidId);
        }

        // Ids have been validated
        let student_list = unsafe {
            student_list
                .into_iter()
                .map(|(key, value)| (StudentId::new(key), value))
                .collect()
        };
        let periods = unsafe { Periods::from_external_data(periods) };
        let subjects = unsafe { Subjects::from_external_data(subjects) };

        let data = Data {
            id_issuer,
            inner_data: InnerData {
                student_list,
                periods,
                subjects,
            },
        };

        assert!(data.check_invariants());

        Ok(data)
    }

    /// Get the student list
    pub fn get_student_list(&self) -> &BTreeMap<StudentId, PersonWithContact> {
        &self.inner_data.student_list
    }

    /// Return the description of the periods
    pub fn get_periods(&self) -> &periods::Periods {
        &self.inner_data.periods
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
    /// Apply period operations
    fn apply_period(&mut self, period_op: &AnnotatedPeriodOp) -> std::result::Result<(), Error> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                self.inner_data.periods.first_week = new_date.clone();
                Ok(())
            }
            AnnotatedPeriodOp::AddFront(period_id, desc) => {
                if self
                    .inner_data
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(Error::PeriodIdAlreadyExists(*period_id));
                }

                self.inner_data
                    .periods
                    .ordered_period_list
                    .insert(0, (*period_id, desc.clone()));
                Ok(())
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id, desc) => {
                if self
                    .inner_data
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(Error::PeriodIdAlreadyExists(*period_id));
                }

                let Some(position) = self.inner_data.periods.find_period_position(*after_id) else {
                    return Err(Error::InvalidPeriodId(*after_id));
                };

                self.inner_data
                    .periods
                    .ordered_period_list
                    .insert(position + 1, (*period_id, desc.clone()));
                Ok(())
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(Error::InvalidPeriodId(*period_id));
                };

                self.inner_data.periods.ordered_period_list.remove(position);

                Ok(())
            }
            AnnotatedPeriodOp::Update(period_id, desc) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(Error::InvalidPeriodId(*period_id));
                };

                self.inner_data.periods.ordered_period_list[position].1 = desc.clone();

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

    /// Used internally
    ///
    /// Builds reverse of a period operation
    fn build_rev_period(
        &self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, Error> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(_new_date) => Ok(
                AnnotatedPeriodOp::ChangeStartDate(self.inner_data.periods.first_week.clone()),
            ),
            AnnotatedPeriodOp::AddFront(new_id, _desc) => {
                if self
                    .inner_data
                    .periods
                    .find_period_position(*new_id)
                    .is_some()
                {
                    return Err(Error::PeriodIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedPeriodOp::Remove(new_id.clone()))
            }
            AnnotatedPeriodOp::AddAfter(new_id, after_id, _desc) => {
                if self
                    .inner_data
                    .periods
                    .find_period_position(*new_id)
                    .is_some()
                {
                    return Err(Error::PeriodIdAlreadyExists(new_id.clone()));
                }

                let Some(_after_position) = self.inner_data.periods.find_period_position(*after_id)
                else {
                    return Err(Error::InvalidPeriodId(after_id.clone()));
                };

                Ok(AnnotatedPeriodOp::Remove(new_id.clone()))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(Error::InvalidPeriodId(period_id.clone()));
                };

                let old_desc = self.inner_data.periods.ordered_period_list[position]
                    .1
                    .clone();

                Ok(if position == 0 {
                    AnnotatedPeriodOp::AddFront(period_id.clone(), old_desc)
                } else {
                    let previous_id = self.inner_data.periods.ordered_period_list[position - 1].0;
                    AnnotatedPeriodOp::AddAfter(period_id.clone(), previous_id.clone(), old_desc)
                })
            }
            AnnotatedPeriodOp::Update(period_id, _desc) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(Error::InvalidPeriodId(period_id.clone()));
                };

                let old_desc = self.inner_data.periods.ordered_period_list[position]
                    .1
                    .clone();

                Ok(AnnotatedPeriodOp::Update(period_id.clone(), old_desc))
            }
        }
    }
}
