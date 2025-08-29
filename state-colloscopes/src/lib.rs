//! Colloscopes state crate
//!
//! This crate implements the various concepts of [collomatique-state]
//! and the various traits for the specific case of colloscope representation.
//!

use assignments::{Assignments, AssignmentsExternalData};
use collomatique_state::{tools, InMemoryData, Operation};
use periods::{Periods, PeriodsExternalData};
use slots::Slots;
use slots::SlotsExternalData;
use std::collections::BTreeSet;
use students::{Students, StudentsExternalData};
use subjects::{Subjects, SubjectsExternalData};
use teachers::{Teachers, TeachersExternalData};
use week_patterns::WeekPatterns;
use week_patterns::WeekPatternsExternalData;

pub mod ids;
use ids::IdIssuer;
pub use ids::{PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekPatternId};
pub mod ops;
use ops::{
    AnnotatedAssignmentOp, AnnotatedPeriodOp, AnnotatedStudentOp, AnnotatedSubjectOp,
    AnnotatedTeacherOp, AnnotatedWeekPatternOp,
};
pub use ops::{
    AnnotatedOp, AssignmentOp, Op, PeriodOp, StudentOp, SubjectOp, TeacherOp, WeekPatternOp,
};
pub use subjects::{
    Subject, SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
};

pub mod assignments;
pub mod periods;
pub mod slots;
pub mod students;
pub mod subjects;
pub mod teachers;
pub mod week_patterns;

/// Description of a person with contacts
///
/// This type is used to describe both students and teachers.
/// Each student and teacher has its own card with name and contacts.
/// There are not used for the colloscope solving process
/// but can help produce a nice colloscope output with contact info.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
    periods: periods::Periods,
    subjects: subjects::Subjects,
    teachers: teachers::Teachers,
    students: students::Students,
    assignments: assignments::Assignments,
    week_patterns: week_patterns::WeekPatterns,
    slots: slots::Slots,
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

/// Errors for students operations
///
/// These errors can be returned when trying to modify [Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentError {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Some non-default assignments are still present for the student
    #[error(
        "student id {0:?} has non-default assignments for subject id {1:?} in period id ({0:?}) and cannot be removed or updated"
    )]
    StudentStillHasNonTrivialAssignments(StudentId, SubjectId, PeriodId),
}

/// Errors for periods operations
///
/// These errors can be returned when trying to modify [Data] with a period op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),

    /// The period is referenced by a subject
    #[error("period id ({0:?}) is referenced by subject {1:?}")]
    PeriodIsReferencedBySubject(PeriodId, SubjectId),

    /// The period is referenced by a student
    #[error("period id ({0:?}) is referenced by student {1:?}")]
    PeriodIsReferencedByStudent(PeriodId, StudentId),

    /// Some non-default assignments are still present for the period
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed"
    )]
    PeriodStillHasNonTrivialAssignments(PeriodId, SubjectId),
}

/// Errors for subject operations
///
/// These errors can be returned when trying to modify [Data] with a subject op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SubjectError {
    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The subject id already exists
    #[error("subject id ({0:?}) already exists")]
    SubjectIdAlreadyExists(SubjectId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// A reference period is invalid
    #[error("Referenced period id {0:?} is invalid")]
    InvalidPeriodId(PeriodId),

    /// Invalid parameters : students per group
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,

    /// Invalid parameters : groups per interrogation
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,

    /// Invalid parameters : week block has empty range for interrogation count
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,

    /// The subject is referenced by a teacher
    #[error("subject id ({0:?}) is referenced by teacher {1:?}")]
    SubjectIsReferencedByTeacher(SubjectId, TeacherId),

    /// Some non-default assignments are still present for the subject
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed or updated"
    )]
    SubjectStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some teachers still are associated to the subject
    #[error("teacher id ({0:?}) is associated to the subject id {1:?}")]
    SubjectStillHasAssociatedTeachers(TeacherId, SubjectId),
}

/// Errors for teacher operations
///
/// These errors can be returned when trying to modify [Data] with a teacher op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum TeacherError {
    /// A teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// The teacher id already exists
    #[error("teacher id ({0:?}) already exists")]
    TeacherIdAlreadyExists(TeacherId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The selected subject does not have interrogations
    #[error("Subject id ({0:?}) corresponds to a subject without interrogations")]
    SubjectHasNoInterrogation(SubjectId),
}

/// Errors for assignment operations
///
/// These errors can be returned when trying to modify [Data] with a assignment op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(SubjectId, PeriodId),

    /// Student is not present on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    StudentIsNotPresentOnPeriod(StudentId, PeriodId),
}

/// Errors for week pattern operations
///
/// These errors can be returned when trying to modify [Data] with a week pattern op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPatternError {
    /// A week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// The week pattern id already exists
    #[error("week pattern id ({0:?}) already exists")]
    WeekPatternIdAlreadyExists(WeekPatternId),
}

/// Errors for colloscopes modification
///
/// These errors can be returned when trying to modify [Data].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    Student(#[from] StudentError),
    #[error(transparent)]
    Period(#[from] PeriodError),
    #[error(transparent)]
    Subject(#[from] SubjectError),
    #[error(transparent)]
    Teacher(#[from] TeacherError),
    #[error(transparent)]
    Assignment(#[from] AssignmentError),
    #[error(transparent)]
    WeekPattern(#[from] WeekPatternError),
}

/// Errors for IDs
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FromDataError {
    #[error(transparent)]
    IdError(#[from] tools::IdError),
    #[error("Inconsistent assignments")]
    InconsistentAssignments,
    #[error("Error in slots data")]
    InconsistentSlots,
}

/// Potential new id returned by annotation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewId {
    StudentId(StudentId),
    PeriodId(PeriodId),
    SubjectId(SubjectId),
    TeacherId(TeacherId),
    WeekPatternId(WeekPatternId),
    SlotId(SlotId),
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

impl From<SubjectId> for NewId {
    fn from(value: SubjectId) -> Self {
        NewId::SubjectId(value)
    }
}

impl From<TeacherId> for NewId {
    fn from(value: TeacherId) -> Self {
        NewId::TeacherId(value)
    }
}

impl From<WeekPatternId> for NewId {
    fn from(value: WeekPatternId) -> Self {
        NewId::WeekPatternId(value)
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
            AnnotatedOp::Subject(subject_op) => {
                Ok(AnnotatedOp::Subject(self.build_rev_subject(subject_op)?))
            }
            AnnotatedOp::Teacher(teacher_op) => {
                Ok(AnnotatedOp::Teacher(self.build_rev_teacher(teacher_op)?))
            }
            AnnotatedOp::Assignment(assignment_op) => Ok(AnnotatedOp::Assignment(
                self.build_rev_assignment(assignment_op)?,
            )),
            AnnotatedOp::WeekPattern(week_pattern_op) => Ok(AnnotatedOp::WeekPattern(
                self.build_rev_week_pattern(week_pattern_op)?,
            )),
        }
    }

    fn apply(&mut self, op: &Self::AnnotatedOperation) -> std::result::Result<(), Self::Error> {
        match op {
            AnnotatedOp::Student(student_op) => self.apply_student(student_op)?,
            AnnotatedOp::Period(period_op) => self.apply_period(period_op)?,
            AnnotatedOp::Subject(subject_op) => self.apply_subject(subject_op)?,
            AnnotatedOp::Teacher(teacher_op) => self.apply_teacher(teacher_op)?,
            AnnotatedOp::Assignment(assignment_op) => self.apply_assignment(assignment_op)?,
            AnnotatedOp::WeekPattern(week_pattern_op) => {
                self.apply_week_pattern(week_pattern_op)?
            }
        }
        self.check_invariants();
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

        if !self
            .inner_data
            .students
            .student_map
            .contains_key(&student_id)
        {
            return None;
        }

        Some(student_id)
    }

    /// Promotes an u64 to a [SubjectId] if it is valid
    pub fn validate_subject_id(&self, id: u64) -> Option<SubjectId> {
        for (subject_id, _) in &self.inner_data.subjects.ordered_subject_list {
            if subject_id.inner() == id {
                return Some(*subject_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [TeacherId] if it is valid
    pub fn validate_teacher_id(&self, id: u64) -> Option<TeacherId> {
        let temp_teacher_id = unsafe { TeacherId::new(id) };
        if self
            .inner_data
            .teachers
            .teacher_map
            .contains_key(&temp_teacher_id)
        {
            return Some(temp_teacher_id);
        }

        None
    }

    /// Promotes an u64 to a [WeekPatternId] if it is valid
    pub fn validate_week_pattern_id(&self, id: u64) -> Option<WeekPatternId> {
        let temp_week_pattern_id = unsafe { WeekPatternId::new(id) };
        if self
            .inner_data
            .week_patterns
            .week_pattern_map
            .contains_key(&temp_week_pattern_id)
        {
            return Some(temp_week_pattern_id);
        }

        None
    }

    /// Promotes a [teachers::TeacherExternalData] to a [teachers::Teacher] if it is valid
    pub fn promote_teacher(
        &self,
        teacher: teachers::TeacherExternalData,
    ) -> Result<teachers::Teacher, u64> {
        let mut new_subjects = BTreeSet::new();

        for subject_id in teacher.subjects {
            let Some(validated_id) = self.validate_subject_id(subject_id) else {
                return Err(subject_id);
            };
            new_subjects.insert(validated_id);
        }

        Ok(teachers::Teacher {
            desc: teacher.desc,
            subjects: new_subjects,
        })
    }

    /// Promotes a [students::StudentExternalData] to a [students::Student] if it is valid
    pub fn promote_student(
        &self,
        student: students::StudentExternalData,
    ) -> Result<students::Student, u64> {
        let mut new_excluded_periods = BTreeSet::new();

        for period_id in student.excluded_periods {
            let Some(validated_id) = self.validate_period_id(period_id) else {
                return Err(period_id);
            };
            new_excluded_periods.insert(validated_id);
        }

        Ok(students::Student {
            desc: student.desc,
            excluded_periods: new_excluded_periods,
        })
    }
}

impl Data {
    /// USED INTERNALLY
    ///
    /// Checks that there are no duplicate ids in data
    ///
    /// Even ids for different type of data should be different
    fn check_no_duplicate_ids(&self) {
        let mut ids_so_far = BTreeSet::new();

        for (id, _) in &self.inner_data.periods.ordered_period_list {
            assert!(ids_so_far.insert(id.inner()));
        }

        for (id, _) in &self.inner_data.subjects.ordered_subject_list {
            assert!(ids_so_far.insert(id.inner()));
        }

        for (id, _) in &self.inner_data.students.student_map {
            assert!(ids_so_far.insert(id.inner()));
        }

        for (id, _) in &self.inner_data.teachers.teacher_map {
            assert!(ids_so_far.insert(id.inner()));
        }

        for (id, _) in &self.inner_data.week_patterns.week_pattern_map {
            assert!(ids_so_far.insert(id.inner()));
        }

        for (_subject_id, subject_slots) in &self.inner_data.slots.subject_map {
            for (id, _) in &subject_slots.ordered_slots {
                assert!(ids_so_far.insert(id.inner()));
            }
        }
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_subject_internal(
        subject: &subjects::Subject,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), SubjectError> {
        for period_id in &subject.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(SubjectError::InvalidPeriodId(*period_id));
            }
        }

        let Some(interrogation_parameters) = &subject.parameters.interrogation_parameters else {
            return Ok(());
        };

        if interrogation_parameters.students_per_group.is_empty() {
            return Err(SubjectError::StudentsPerGroupRangeIsEmpty);
        }
        if interrogation_parameters.groups_per_interrogation.is_empty() {
            return Err(SubjectError::GroupsPerInterrogationRangeIsEmpty);
        }

        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks,
                minimum_week_separation: _,
            } => {
                for block in blocks {
                    if block.interrogation_count_in_block.is_empty() {
                        return Err(SubjectError::InterrogationCountRangeIsEmpty);
                    }
                }
            }
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation: _,
            } => {
                if interrogation_count_in_year.is_empty() {
                    return Err(SubjectError::InterrogationCountRangeIsEmpty);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a subject before commiting a subject op
    fn validate_subject(&self, subject: &subjects::Subject) -> Result<(), SubjectError> {
        let period_ids = self.build_period_ids();

        Self::validate_subject_internal(subject, &period_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_subjects_data_consistency(&self, period_ids: &BTreeSet<PeriodId>) {
        for (_subject_id, subject) in &self.inner_data.subjects.ordered_subject_list {
            Self::validate_subject_internal(subject, period_ids).unwrap();
        }
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_teacher_internal(
        teacher: &teachers::Teacher,
        subjects: &subjects::Subjects,
    ) -> Result<(), TeacherError> {
        for subject_id in &teacher.subjects {
            let Some(subject) = subjects.find_subject(*subject_id) else {
                return Err(TeacherError::InvalidSubjectId(*subject_id));
            };
            if subject.parameters.interrogation_parameters.is_none() {
                return Err(TeacherError::SubjectHasNoInterrogation(*subject_id));
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    fn validate_teacher(&self, teacher: &teachers::Teacher) -> Result<(), TeacherError> {
        Self::validate_teacher_internal(teacher, &self.inner_data.subjects)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_teachers_data_consistency(&self) {
        for (_teacher_id, teacher) in &self.inner_data.teachers.teacher_map {
            Self::validate_teacher_internal(teacher, &self.inner_data.subjects).unwrap();
        }
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_student_internal(
        student: &students::Student,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), StudentError> {
        for period_id in &student.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(StudentError::InvalidPeriodId(*period_id));
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    fn validate_student(&self, student: &students::Student) -> Result<(), StudentError> {
        let period_ids = self.build_period_ids();

        Self::validate_student_internal(student, &period_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_students_data_consistency(&self, period_ids: &BTreeSet<PeriodId>) {
        for (_student_id, student) in &self.inner_data.students.student_map {
            Self::validate_student_internal(student, period_ids).unwrap();
        }
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in assignments data
    fn check_assignments_data_consistency(&self, period_ids: &BTreeSet<PeriodId>) {
        assert!(self.inner_data.assignments.period_map.len() == period_ids.len());
        for (period_id, period_assignments) in &self.inner_data.assignments.period_map {
            assert!(period_ids.contains(period_id));

            let mut subject_count_for_period = 0usize;
            for (subject_id, subject) in &self.inner_data.subjects.ordered_subject_list {
                if subject.excluded_periods.contains(period_id) {
                    continue;
                }
                subject_count_for_period += 1;

                let subject_assignments = period_assignments
                    .subject_map
                    .get(subject_id)
                    .expect("All relevant subjects for the period should appear in the map");

                for student_id in subject_assignments {
                    let student = self
                        .inner_data
                        .students
                        .student_map
                        .get(student_id)
                        .expect("Every student that appears in the map should be a valid id");

                    if student.excluded_periods.contains(period_id) {
                        panic!(
                            "Assigned student {:?} is not present for period {:?}",
                            student_id, period_id
                        );
                    }
                }
            }
            assert!(subject_count_for_period == period_assignments.subject_map.len());
        }
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
    /// Checks all the invariants of data
    fn check_invariants(&self) {
        self.check_no_duplicate_ids();

        let period_ids = self.build_period_ids();

        self.check_subjects_data_consistency(&period_ids);
        self.check_teachers_data_consistency();
        self.check_students_data_consistency(&period_ids);
        self.check_assignments_data_consistency(&period_ids);
    }
}

impl Data {
    /// Create a new [Data]
    ///
    /// This [Data] is basically empty and corresponds to the
    /// state of a new file
    pub fn new() -> Data {
        Self::from_data(
            PeriodsExternalData::default(),
            SubjectsExternalData::default(),
            TeachersExternalData::default(),
            StudentsExternalData::default(),
            AssignmentsExternalData::default(),
            WeekPatternsExternalData::default(),
            SlotsExternalData::default(),
        )
        .expect("Default data should be valid")
    }

    /// Create a new [Data] from existing data
    ///
    /// This will check the consistency of the data
    /// and will also do some internal checks, so this might fail.
    pub fn from_data(
        periods: periods::PeriodsExternalData,
        subjects: subjects::SubjectsExternalData,
        teachers: teachers::TeachersExternalData,
        students: students::StudentsExternalData,
        assignments: assignments::AssignmentsExternalData,
        week_patterns: week_patterns::WeekPatternsExternalData,
        slots: slots::SlotsExternalData,
    ) -> Result<Data, FromDataError> {
        let student_ids = students.student_map.keys().copied();
        let period_ids = periods.ordered_period_list.iter().map(|(id, _d)| *id);
        let subject_ids = subjects.ordered_subject_list.iter().map(|(id, _d)| *id);
        let teacher_ids = teachers.teacher_map.keys().copied();
        let week_patterns_ids = week_patterns.week_pattern_map.keys().copied();
        let slot_ids = slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots.ordered_slots.iter().map(|(id, _d)| *id)
            });
        let id_issuer = IdIssuer::new(
            student_ids,
            period_ids,
            subject_ids,
            teacher_ids,
            week_patterns_ids,
            slot_ids,
        )?;

        let period_ids: std::collections::BTreeSet<_> = periods
            .ordered_period_list
            .iter()
            .map(|(id, _d)| *id)
            .collect();
        let week_pattern_ids: std::collections::BTreeSet<_> =
            week_patterns.week_pattern_map.keys().copied().collect();
        if !subjects.validate_all(&period_ids) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !teachers.validate_all(&subjects) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !students.validate_all(&period_ids) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !assignments.validate_all(&period_ids, &students, &subjects) {
            return Err(FromDataError::InconsistentAssignments);
        }
        if !slots.validate_all(&subjects, &week_pattern_ids, &teachers) {
            return Err(FromDataError::InconsistentSlots);
        }

        // Ids have been validated
        let students = unsafe { Students::from_external_data(students) };
        let periods = unsafe { Periods::from_external_data(periods) };
        let subjects = unsafe { Subjects::from_external_data(subjects) };
        let teachers = unsafe { Teachers::from_external_data(teachers) };
        let assignments = unsafe { Assignments::from_external_data(assignments) };
        let week_patterns = unsafe { WeekPatterns::from_external_data(week_patterns) };
        let slots = unsafe { Slots::from_external_data(slots) };

        let data = Data {
            id_issuer,
            inner_data: InnerData {
                periods,
                subjects,
                teachers,
                students,
                assignments,
                week_patterns,
                slots,
            },
        };

        data.check_invariants();

        Ok(data)
    }

    /// Get the students
    pub fn get_students(&self) -> &students::Students {
        &self.inner_data.students
    }

    /// Get the subjects
    pub fn get_subjects(&self) -> &subjects::Subjects {
        &self.inner_data.subjects
    }

    /// Return the description of the periods
    pub fn get_periods(&self) -> &periods::Periods {
        &self.inner_data.periods
    }

    /// Get the subjects
    pub fn get_teachers(&self) -> &teachers::Teachers {
        &self.inner_data.teachers
    }

    /// Get the assignments
    pub fn get_assignments(&self) -> &assignments::Assignments {
        &self.inner_data.assignments
    }

    /// Get the week patterns
    pub fn get_week_patterns(&self) -> &week_patterns::WeekPatterns {
        &self.inner_data.week_patterns
    }

    /// Get the slots
    pub fn get_slots(&self) -> &slots::Slots {
        &self.inner_data.slots
    }

    /// Used internally
    ///
    /// Apply student operations
    fn apply_student(
        &mut self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<(), StudentError> {
        match student_op {
            AnnotatedStudentOp::Add(new_id, student) => {
                if self.inner_data.students.student_map.get(new_id).is_some() {
                    return Err(StudentError::StudentIdAlreadyExists(*new_id));
                }
                self.validate_student(student)?;

                self.inner_data
                    .students
                    .student_map
                    .insert(*new_id, student.clone());

                Ok(())
            }
            AnnotatedStudentOp::Remove(id) => {
                let Some(current_student) = self.inner_data.students.student_map.get(id) else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (period_id, period_assignments) in &self.inner_data.assignments.period_map {
                    if current_student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                self.inner_data.students.student_map.remove(id);

                Ok(())
            }
            AnnotatedStudentOp::Update(id, new_student) => {
                self.validate_student(new_student)?;
                let Some(current_student) = self.inner_data.students.student_map.get_mut(id) else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (period_id, period_assignments) in &self.inner_data.assignments.period_map {
                    if current_student.excluded_periods.contains(period_id)
                        || !new_student.excluded_periods.contains(period_id)
                    {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                *current_student = new_student.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<(), PeriodError> {
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
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                self.inner_data
                    .periods
                    .ordered_period_list
                    .insert(0, (*period_id, desc.clone()));
                self.inner_data.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                Ok(())
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id, desc) => {
                if self
                    .inner_data
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                let Some(position) = self.inner_data.periods.find_period_position(*after_id) else {
                    return Err(PeriodError::InvalidPeriodId(*after_id));
                };

                self.inner_data
                    .periods
                    .ordered_period_list
                    .insert(position + 1, (*period_id, desc.clone()));
                self.inner_data.assignments.period_map.insert(
                    *period_id,
                    assignments::PeriodAssignments {
                        subject_map: self
                            .inner_data
                            .subjects
                            .ordered_subject_list
                            .iter()
                            .map(|(subject_id, _subject)| (*subject_id, BTreeSet::new()))
                            .collect(),
                    },
                );
                Ok(())
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                for (subject_id, subject) in &self.inner_data.subjects.ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySubject(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                for (student_id, student) in &self.inner_data.students.student_map {
                    if student.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByStudent(
                            *period_id,
                            *student_id,
                        ));
                    }
                }

                let period_assignments = self
                    .inner_data
                    .assignments
                    .period_map
                    .get(period_id)
                    .expect("At this point, period id should be valid");
                for (subject_id, assigned_students) in &period_assignments.subject_map {
                    if !assigned_students.is_empty() {
                        return Err(PeriodError::PeriodStillHasNonTrivialAssignments(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                self.inner_data.periods.ordered_period_list.remove(position);
                self.inner_data.assignments.period_map.remove(period_id);

                Ok(())
            }
            AnnotatedPeriodOp::Update(period_id, desc) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                self.inner_data.periods.ordered_period_list[position].1 = desc.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply period operations
    fn apply_subject(
        &mut self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<(), SubjectError> {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, params) => {
                if self
                    .inner_data
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(*new_id));
                }
                self.validate_subject(params)?;

                let position = match after_id {
                    Some(id) => {
                        self.inner_data
                            .subjects
                            .find_subject_position(*id)
                            .ok_or(SubjectError::InvalidSubjectId(*id))?
                            + 1
                    }
                    None => 0,
                };

                self.inner_data
                    .subjects
                    .ordered_subject_list
                    .insert(position, (*new_id, params.clone()));
                for (period_id, _period) in &self.inner_data.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment
                        .subject_map
                        .insert(*new_id, BTreeSet::new());
                }

                Ok(())
            }
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos >= self.inner_data.subjects.ordered_subject_list.len() {
                    return Err(SubjectError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data.subjects.ordered_subject_list.len(),
                    ));
                }
                let Some(old_pos) = self.inner_data.subjects.find_subject_position(*id) else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let data = self
                    .inner_data
                    .subjects
                    .ordered_subject_list
                    .remove(old_pos);
                self.inner_data
                    .subjects
                    .ordered_subject_list
                    .insert(*new_pos, data);
                Ok(())
            }
            AnnotatedSubjectOp::Remove(id) => {
                let Some(position) = self.inner_data.subjects.find_subject_position(*id) else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                for (teacher_id, teacher) in &self.inner_data.teachers.teacher_map {
                    if teacher.subjects.contains(id) {
                        return Err(SubjectError::SubjectIsReferencedByTeacher(*id, *teacher_id));
                    }
                }

                let params = &self.inner_data.subjects.ordered_subject_list[position].1;
                for (period_id, _period) in &self.inner_data.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .assignments
                        .period_map
                        .get(period_id)
                        .expect("Every period should appear in assignments");

                    let assigned_students = period_assignment
                        .subject_map
                        .get(id)
                        .expect("Subject should appear in assignments for relevant periods");

                    if !assigned_students.is_empty() {
                        return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                            *period_id, *id,
                        ));
                    }
                }

                let (_, params) = self
                    .inner_data
                    .subjects
                    .ordered_subject_list
                    .remove(position);
                for (period_id, _period) in &self.inner_data.periods.ordered_period_list {
                    if params.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .assignments
                        .period_map
                        .get_mut(period_id)
                        .expect("Every period should appear in assignments");

                    period_assignment.subject_map.remove(id);
                }

                Ok(())
            }
            AnnotatedSubjectOp::Update(id, new_params) => {
                self.validate_subject(new_params)?;
                let Some(position) = self.inner_data.subjects.find_subject_position(*id) else {
                    return Err(SubjectError::InvalidSubjectId(*id));
                };

                let old_params = self.inner_data.subjects.ordered_subject_list[position]
                    .1
                    .clone();

                if old_params.parameters.interrogation_parameters.is_some()
                    && new_params.parameters.interrogation_parameters.is_none()
                {
                    // The new subject does not have interrogations, let's check that no teacher has been assigned to it
                    for (teacher_id, teacher) in &self.inner_data.teachers.teacher_map {
                        if teacher.subjects.contains(id) {
                            return Err(SubjectError::SubjectStillHasAssociatedTeachers(
                                *teacher_id,
                                *id,
                            ));
                        }
                    }
                }

                for (period_id, _period) in &self.inner_data.periods.ordered_period_list {
                    // If the period was excluded before, there is no structure to check
                    // and if the period is not excluded now, the structure will be fine anyway
                    if old_params.excluded_periods.contains(period_id)
                        || !new_params.excluded_periods.contains(period_id)
                    {
                        continue;
                    }

                    let period_assignment = self
                        .inner_data
                        .assignments
                        .period_map
                        .get(period_id)
                        .expect("Every period should appear in assignments");

                    let assigned_students = period_assignment
                        .subject_map
                        .get(id)
                        .expect("Subject should appear in assignments for relevant periods");

                    if !assigned_students.is_empty() {
                        return Err(SubjectError::SubjectStillHasNonTrivialAssignments(
                            *period_id, *id,
                        ));
                    }
                }

                self.inner_data.subjects.ordered_subject_list[position].1 = new_params.clone();

                for (period_id, _period) in &self.inner_data.periods.ordered_period_list {
                    // Only change in period status should be considered
                    if old_params.excluded_periods.contains(period_id)
                        == new_params.excluded_periods.contains(period_id)
                    {
                        continue;
                    }

                    if old_params.excluded_periods.contains(period_id) {
                        // The period was excluded but is not anymore
                        let period_assignment = self
                            .inner_data
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.insert(*id, BTreeSet::new());
                    } else {
                        // The period was included but will now be excluded
                        let period_assignment = self
                            .inner_data
                            .assignments
                            .period_map
                            .get_mut(period_id)
                            .expect("Every period should appear in assignments");

                        period_assignment.subject_map.remove(id);
                    }
                }

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_teacher(
        &mut self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<(), TeacherError> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, teacher) => {
                if self.inner_data.teachers.teacher_map.get(new_id).is_some() {
                    return Err(TeacherError::TeacherIdAlreadyExists(*new_id));
                }
                self.validate_teacher(teacher)?;

                self.inner_data
                    .teachers
                    .teacher_map
                    .insert(*new_id, teacher.clone());

                Ok(())
            }
            AnnotatedTeacherOp::Remove(id) => {
                if !self.inner_data.teachers.teacher_map.contains_key(id) {
                    return Err(TeacherError::InvalidTeacherId(*id));
                }

                self.inner_data.teachers.teacher_map.remove(id);

                Ok(())
            }
            AnnotatedTeacherOp::Update(id, new_teacher) => {
                self.validate_teacher(new_teacher)?;
                let Some(current_teacher) = self.inner_data.teachers.teacher_map.get_mut(id) else {
                    return Err(TeacherError::InvalidTeacherId(*id));
                };

                *current_teacher = new_teacher.clone();

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_assignment(
        &mut self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<(), AssignmentError> {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, status) => {
                let Some(period_assignments) =
                    self.inner_data.assignments.period_map.get_mut(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                }

                let Some(assigned_students) = period_assignments.subject_map.get_mut(subject_id)
                else {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                };

                let Some(student_desc) = self.inner_data.students.student_map.get(student_id)
                else {
                    return Err(AssignmentError::InvalidStudentId(*student_id));
                };

                if student_desc.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::StudentIsNotPresentOnPeriod(
                        *student_id,
                        *period_id,
                    ));
                }

                if *status {
                    assigned_students.insert(*student_id);
                } else {
                    assigned_students.remove(student_id);
                }

                Ok(())
            }
        }
    }

    /// Used internally
    ///
    /// Apply teacher operations
    fn apply_week_pattern(
        &mut self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<(), WeekPatternError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, week_pattern) => {
                if self
                    .inner_data
                    .week_patterns
                    .week_pattern_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(*new_id));
                }

                self.inner_data
                    .week_patterns
                    .week_pattern_map
                    .insert(*new_id, week_pattern.clone());

                Ok(())
            }
            AnnotatedWeekPatternOp::Remove(id) => {
                if !self
                    .inner_data
                    .week_patterns
                    .week_pattern_map
                    .contains_key(id)
                {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                }

                self.inner_data.week_patterns.week_pattern_map.remove(id);

                Ok(())
            }
            AnnotatedWeekPatternOp::Update(id, new_week_pattern) => {
                let Some(current_week_pattern) =
                    self.inner_data.week_patterns.week_pattern_map.get_mut(id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(*id));
                };

                *current_week_pattern = new_week_pattern.clone();

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
    ) -> std::result::Result<AnnotatedStudentOp, StudentError> {
        match student_op {
            AnnotatedStudentOp::Add(student_id, _student) => {
                if self
                    .inner_data
                    .students
                    .student_map
                    .contains_key(student_id)
                {
                    return Err(StudentError::StudentIdAlreadyExists(student_id.clone()));
                }

                Ok(AnnotatedStudentOp::Remove(student_id.clone()))
            }
            AnnotatedStudentOp::Remove(student_id) => {
                let Some(old_student) = self
                    .inner_data
                    .students
                    .student_map
                    .get(&student_id)
                    .cloned()
                else {
                    return Err(StudentError::InvalidStudentId(student_id.clone()));
                };

                Ok(AnnotatedStudentOp::Add(student_id.clone(), old_student))
            }
            AnnotatedStudentOp::Update(student_id, _student) => {
                let Some(old_student) = self
                    .inner_data
                    .students
                    .student_map
                    .get(&student_id)
                    .cloned()
                else {
                    return Err(StudentError::InvalidStudentId(student_id.clone()));
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
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodError> {
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
                    return Err(PeriodError::PeriodIdAlreadyExists(new_id.clone()));
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
                    return Err(PeriodError::PeriodIdAlreadyExists(new_id.clone()));
                }

                let Some(_after_position) = self.inner_data.periods.find_period_position(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(after_id.clone()));
                };

                Ok(AnnotatedPeriodOp::Remove(new_id.clone()))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self.inner_data.periods.find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(period_id.clone()));
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
                    return Err(PeriodError::InvalidPeriodId(period_id.clone()));
                };

                let old_desc = self.inner_data.periods.ordered_period_list[position]
                    .1
                    .clone();

                Ok(AnnotatedPeriodOp::Update(period_id.clone(), old_desc))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a subject operation
    fn build_rev_subject(
        &self,
        subject_op: &AnnotatedSubjectOp,
    ) -> std::result::Result<AnnotatedSubjectOp, SubjectError> {
        match subject_op {
            AnnotatedSubjectOp::AddAfter(new_id, after_id, _params) => {
                if self
                    .inner_data
                    .subjects
                    .find_subject_position(*new_id)
                    .is_some()
                {
                    return Err(SubjectError::SubjectIdAlreadyExists(new_id.clone()));
                }

                if let Some(id) = after_id {
                    if self
                        .inner_data
                        .subjects
                        .find_subject_position(*id)
                        .is_none()
                    {
                        return Err(SubjectError::InvalidSubjectId(id.clone()));
                    }
                }

                Ok(AnnotatedSubjectOp::Remove(new_id.clone()))
            }
            AnnotatedSubjectOp::Remove(subject_id) => {
                let Some(position) = self.inner_data.subjects.find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(subject_id.clone()));
                };

                let old_params = self.inner_data.subjects.ordered_subject_list[position]
                    .1
                    .clone();

                Ok(AnnotatedSubjectOp::AddAfter(
                    *subject_id,
                    if position == 0 {
                        None
                    } else {
                        Some(self.inner_data.subjects.ordered_subject_list[position - 1].0)
                    },
                    old_params.into(),
                ))
            }
            AnnotatedSubjectOp::Update(subject_id, _new_params) => {
                let Some(position) = self.inner_data.subjects.find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*subject_id));
                };

                let old_params = self.inner_data.subjects.ordered_subject_list[position]
                    .1
                    .clone();

                Ok(AnnotatedSubjectOp::Update(*subject_id, old_params.into()))
            }
            AnnotatedSubjectOp::ChangePosition(subject_id, _new_pos) => {
                let Some(old_pos) = self.inner_data.subjects.find_subject_position(*subject_id)
                else {
                    return Err(SubjectError::InvalidSubjectId(*subject_id));
                };

                Ok(AnnotatedSubjectOp::ChangePosition(*subject_id, old_pos))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a teacher operation
    fn build_rev_teacher(
        &self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<AnnotatedTeacherOp, TeacherError> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, _teacher) => {
                if self.inner_data.teachers.teacher_map.get(new_id).is_some() {
                    return Err(TeacherError::TeacherIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedTeacherOp::Remove(new_id.clone()))
            }
            AnnotatedTeacherOp::Remove(teacher_id) => {
                let Some(old_teacher) = self.inner_data.teachers.teacher_map.get(teacher_id) else {
                    return Err(TeacherError::InvalidTeacherId(teacher_id.clone()));
                };

                Ok(AnnotatedTeacherOp::Add(*teacher_id, old_teacher.clone()))
            }
            AnnotatedTeacherOp::Update(teacher_id, _new_teacher) => {
                let Some(old_teacher) = self.inner_data.teachers.teacher_map.get(teacher_id) else {
                    return Err(TeacherError::InvalidTeacherId(teacher_id.clone()));
                };

                Ok(AnnotatedTeacherOp::Update(*teacher_id, old_teacher.clone()))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of an assignment operation
    fn build_rev_assignment(
        &self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<AnnotatedAssignmentOp, AssignmentError> {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, _status) => {
                let Some(period_assignments) =
                    self.inner_data.assignments.period_map.get(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                }

                let Some(assigned_students) = period_assignments.subject_map.get(subject_id) else {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                };

                let Some(student_desc) = self.inner_data.students.student_map.get(student_id)
                else {
                    return Err(AssignmentError::InvalidStudentId(*student_id));
                };

                if student_desc.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::StudentIsNotPresentOnPeriod(
                        *student_id,
                        *period_id,
                    ));
                }

                let previous_status = assigned_students.contains(student_id);

                Ok(AnnotatedAssignmentOp::Assign(
                    *period_id,
                    *student_id,
                    *subject_id,
                    previous_status,
                ))
            }
        }
    }

    /// Used internally
    ///
    /// Builds reverse of a teacher operation
    fn build_rev_week_pattern(
        &self,
        week_pattern_op: &AnnotatedWeekPatternOp,
    ) -> std::result::Result<AnnotatedWeekPatternOp, WeekPatternError> {
        match week_pattern_op {
            AnnotatedWeekPatternOp::Add(new_id, _week_pattern) => {
                if self
                    .inner_data
                    .week_patterns
                    .week_pattern_map
                    .get(new_id)
                    .is_some()
                {
                    return Err(WeekPatternError::WeekPatternIdAlreadyExists(new_id.clone()));
                }

                Ok(AnnotatedWeekPatternOp::Remove(new_id.clone()))
            }
            AnnotatedWeekPatternOp::Remove(week_pattern_id) => {
                let Some(old_week_pattern) = self
                    .inner_data
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(
                        week_pattern_id.clone(),
                    ));
                };

                Ok(AnnotatedWeekPatternOp::Add(
                    *week_pattern_id,
                    old_week_pattern.clone(),
                ))
            }
            AnnotatedWeekPatternOp::Update(week_pattern_id, _new_week_pattern) => {
                let Some(old_week_pattern) = self
                    .inner_data
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                else {
                    return Err(WeekPatternError::InvalidWeekPatternId(
                        week_pattern_id.clone(),
                    ));
                };

                Ok(AnnotatedWeekPatternOp::Update(
                    *week_pattern_id,
                    old_week_pattern.clone(),
                ))
            }
        }
    }
}
