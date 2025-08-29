pub mod sqlite;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CheckedError<T, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    CheckData: std::fmt::Debug,
{
    #[error("Check failed. Data provided is: {0:?}")]
    CheckFailed(CheckData),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum IdError<T, Id>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Check failed. Data provided is: {0:?}")]
    CheckFailed(CheckData),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

impl<T, Id, CheckData> CheckedIdError<T, Id, CheckData>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CheckData: std::fmt::Debug,
{
    fn from_id_error(id_error: IdError<T, Id>) -> Self {
        match id_error {
            IdError::InvalidId(id) => CheckedIdError::InvalidId(id),
            IdError::InternalError(int_err) => CheckedIdError::InternalError(int_err),
        }
    }
}

#[derive(Error, Debug)]
pub enum Id2Error<T, Id1, Id2>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossId3Error<T, Id1, Id2, Id3, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id1: std::fmt::Debug,
    Id2: std::fmt::Debug,
    Id3: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId1(Id1),
    #[error("Id {0:?} is invalid")]
    InvalidId2(Id2),
    #[error("Id {0:?} is invalid")]
    InvalidId3(Id3),
    #[error("Id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossError<T, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum CrossIdError<T, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross2Error<T, CrossId1, CrossId2>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross2IdError<T, Id, CrossId1, CrossId2>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3Error<T, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId3: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3IdError<T, Id, CrossId1, CrossId2, CrossId3>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId3: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum Cross3IdWithDepError<T, Id, CrossId1, CrossId2, CrossId3, DepId>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
    CrossId1: std::fmt::Debug,
    CrossId2: std::fmt::Debug,
    CrossId3: std::fmt::Debug,
    DepId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId1(CrossId1),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId2(CrossId2),
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId3(CrossId3),
    #[error("Dependancy of id {0:?} blocks modification of this data")]
    BlockingDependancy(DepId),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossError<T, Data, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Error, Debug)]
pub enum InvalidCrossIdError<T, Data, Id, CrossId>
where
    T: std::fmt::Debug + std::error::Error,
    Data: std::fmt::Debug,
    Id: std::fmt::Debug,
    CrossId: std::fmt::Debug,
{
    #[error("Cross id {0:?} is invalid")]
    InvalidCrossId(CrossId),
    #[error("Data to be stored is invalid: {0:?}")]
    InvalidData(Data),
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

pub trait OrdId:
    std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy
{
}
impl<T: std::fmt::Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy> OrdId
    for T
{
}

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::RangeInclusive;

#[trait_variant::make(Send)]
pub trait Storage: Send + Sync + std::fmt::Debug {
    type WeekPatternId: OrdId;
    type TeacherId: OrdId;
    type StudentId: OrdId;
    type SubjectGroupId: OrdId;
    type IncompatId: OrdId;
    type GroupListId: OrdId;
    type SubjectId: OrdId;
    type TimeSlotId: OrdId;
    type GroupingId: OrdId;
    type GroupingIncompatId: OrdId;
    type ColloscopeId: OrdId;
    type SlotSelectionId: OrdId;

    type InternalError: std::fmt::Debug + std::error::Error + Send;

    async unsafe fn general_data_set_unchecked(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn general_data_get(&self) -> std::result::Result<GeneralData, Self::InternalError>;

    async fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::WeekPatternId, WeekPattern>, Self::InternalError>;
    async fn week_patterns_get(
        &self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<Self::InternalError, Self::WeekPatternId>>;
    async unsafe fn week_patterns_add_unchecked(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<Self::WeekPatternId, Self::InternalError>;
    async unsafe fn week_patterns_remove_unchecked(
        &mut self,
        index: Self::WeekPatternId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn week_patterns_update_unchecked(
        &mut self,
        index: Self::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::TeacherId, Teacher>, Self::InternalError>;
    async fn teachers_get(
        &self,
        index: Self::TeacherId,
    ) -> std::result::Result<Teacher, IdError<Self::InternalError, Self::TeacherId>>;
    async fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<Self::TeacherId, Self::InternalError>;
    async unsafe fn teachers_remove_unchecked(
        &mut self,
        index: Self::TeacherId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn teachers_update(
        &mut self,
        index: Self::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::TeacherId>>;

    async fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::StudentId, Student>, Self::InternalError>;
    async fn students_get(
        &self,
        index: Self::StudentId,
    ) -> std::result::Result<Student, IdError<Self::InternalError, Self::StudentId>>;
    async fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<Self::StudentId, Self::InternalError>;
    async unsafe fn students_remove_unchecked(
        &mut self,
        index: Self::StudentId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn students_update(
        &mut self,
        index: Self::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::StudentId>>;

    async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<Self::SubjectGroupId, SubjectGroup>, Self::InternalError>;
    async fn subject_groups_get(
        &self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<Self::InternalError, Self::SubjectGroupId>>;
    async fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<Self::SubjectGroupId, Self::InternalError>;
    async unsafe fn subject_groups_remove_unchecked(
        &mut self,
        index: Self::SubjectGroupId,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subject_groups_update(
        &mut self,
        index: Self::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<Self::InternalError, Self::SubjectGroupId>>;

    async fn incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::IncompatId, Incompat<Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn incompats_get(
        &self,
        index: Self::IncompatId,
    ) -> std::result::Result<
        Incompat<Self::WeekPatternId>,
        IdError<Self::InternalError, Self::IncompatId>,
    >;
    async unsafe fn incompats_add_unchecked(
        &mut self,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<Self::IncompatId, Self::InternalError>;
    async unsafe fn incompats_remove_unchecked(
        &mut self,
        index: Self::IncompatId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn incompats_update_unchecked(
        &mut self,
        index: Self::IncompatId,
        incompat: &Incompat<Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn group_lists_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupListId, GroupList<Self::StudentId>>,
        Self::InternalError,
    >;
    async fn group_lists_get(
        &self,
        index: Self::GroupListId,
    ) -> std::result::Result<
        GroupList<Self::StudentId>,
        IdError<Self::InternalError, Self::GroupListId>,
    >;
    async unsafe fn group_lists_add_unchecked(
        &mut self,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<Self::GroupListId, Self::InternalError>;
    async unsafe fn group_lists_remove_unchecked(
        &mut self,
        index: Self::GroupListId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn group_lists_update_unchecked(
        &mut self,
        index: Self::GroupListId,
        group_list: &GroupList<Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<
            Self::SubjectId,
            Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        >,
        Self::InternalError,
    >;
    async fn subjects_get(
        &self,
        index: Self::SubjectId,
    ) -> std::result::Result<
        Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
        IdError<Self::InternalError, Self::SubjectId>,
    >;
    async unsafe fn subjects_add_unchecked(
        &mut self,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<Self::SubjectId, Self::InternalError>;
    async unsafe fn subjects_remove_unchecked(
        &mut self,
        index: Self::SubjectId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn subjects_update_unchecked(
        &mut self,
        index: Self::SubjectId,
        subject: &Subject<Self::SubjectGroupId, Self::IncompatId, Self::GroupListId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::TimeSlotId, TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>>,
        Self::InternalError,
    >;
    async fn time_slots_get(
        &self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
        IdError<Self::InternalError, Self::TimeSlotId>,
    >;
    async unsafe fn time_slots_add_unchecked(
        &mut self,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<Self::TimeSlotId, Self::InternalError>;
    async unsafe fn time_slots_remove_unchecked(
        &mut self,
        index: Self::TimeSlotId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn time_slots_update_unchecked(
        &mut self,
        index: Self::TimeSlotId,
        time_slot: &TimeSlot<Self::SubjectId, Self::TeacherId, Self::WeekPatternId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn groupings_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingId, Grouping<Self::TimeSlotId>>,
        Self::InternalError,
    >;
    async fn groupings_get(
        &self,
        index: Self::GroupingId,
    ) -> std::result::Result<
        Grouping<Self::TimeSlotId>,
        IdError<Self::InternalError, Self::GroupingId>,
    >;
    async unsafe fn groupings_add_unchecked(
        &mut self,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<Self::GroupingId, Self::InternalError>;
    async unsafe fn groupings_remove_unchecked(
        &mut self,
        index: Self::GroupingId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn groupings_update_unchecked(
        &mut self,
        index: Self::GroupingId,
        grouping: &Grouping<Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::GroupingIncompatId, GroupingIncompat<Self::GroupingId>>,
        Self::InternalError,
    >;
    async fn grouping_incompats_get(
        &self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<Self::GroupingId>,
        IdError<Self::InternalError, Self::GroupingIncompatId>,
    >;
    async unsafe fn grouping_incompats_add_unchecked(
        &mut self,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<Self::GroupingIncompatId, Self::InternalError>;
    async unsafe fn grouping_incompats_remove_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn grouping_incompats_update_unchecked(
        &mut self,
        index: Self::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<Self::GroupingId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async unsafe fn subject_group_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
        subject_id: Option<Self::SubjectId>,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn subject_group_for_student_get(
        &self,
        student_id: Self::StudentId,
        subject_group_id: Self::SubjectGroupId,
    ) -> std::result::Result<
        Option<Self::SubjectId>,
        Id2Error<Self::InternalError, Self::StudentId, Self::SubjectGroupId>,
    >;

    async unsafe fn incompat_for_student_set_unchecked(
        &mut self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Self::InternalError>;
    async fn incompat_for_student_get(
        &self,
        student_id: Self::StudentId,
        incompat_id: Self::IncompatId,
    ) -> std::result::Result<bool, Id2Error<Self::InternalError, Self::StudentId, Self::IncompatId>>;

    async fn colloscopes_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::ColloscopeId, Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>>,
        Self::InternalError,
    >;
    async fn colloscopes_get(
        &self,
        index: Self::ColloscopeId,
    ) -> std::result::Result<
        Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
        IdError<Self::InternalError, Self::ColloscopeId>,
    >;
    async unsafe fn colloscopes_add_unchecked(
        &mut self,
        colloscope: &Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
    ) -> std::result::Result<Self::ColloscopeId, Self::InternalError>;
    async unsafe fn colloscopes_remove_unchecked(
        &mut self,
        index: Self::ColloscopeId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn colloscopes_update_unchecked(
        &mut self,
        index: Self::ColloscopeId,
        colloscope: &Colloscope<Self::TeacherId, Self::SubjectId, Self::StudentId>,
    ) -> std::result::Result<(), Self::InternalError>;

    async fn slot_selections_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<Self::SlotSelectionId, SlotSelection<Self::SubjectId, Self::TimeSlotId>>,
        Self::InternalError,
    >;
    async fn slot_selections_get(
        &self,
        index: Self::SlotSelectionId,
    ) -> std::result::Result<
        SlotSelection<Self::SubjectId, Self::TimeSlotId>,
        IdError<Self::InternalError, Self::SlotSelectionId>,
    >;
    async unsafe fn slot_selections_add_unchecked(
        &mut self,
        slot_selection: &SlotSelection<Self::SubjectId, Self::TimeSlotId>,
    ) -> std::result::Result<Self::SlotSelectionId, Self::InternalError>;
    async unsafe fn slot_selections_remove_unchecked(
        &mut self,
        index: Self::SlotSelectionId,
    ) -> std::result::Result<(), Self::InternalError>;
    async unsafe fn slot_selections_update_unchecked(
        &mut self,
        index: Self::SlotSelectionId,
        slot_selection: &SlotSelection<Self::SubjectId, Self::TimeSlotId>,
    ) -> std::result::Result<(), Self::InternalError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostsAdjustments {
    pub max_interrogations_per_day_for_single_student: i32,
    pub max_interrogations_per_day_for_all_students: i32,
    pub interrogations_per_week_range_for_single_student: i32,
    pub interrogations_per_week_range_for_all_students: i32,
    pub balancing: i32,
    pub consecutive_slots: i32,
}

impl Default for CostsAdjustments {
    fn default() -> Self {
        CostsAdjustments {
            max_interrogations_per_day_for_single_student: 1,
            max_interrogations_per_day_for_all_students: 1,
            interrogations_per_week_range_for_single_student: 1,
            interrogations_per_week_range_for_all_students: 1,
            balancing: 1,
            consecutive_slots: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralData {
    pub interrogations_per_week: Option<std::ops::Range<u32>>,
    pub max_interrogations_per_day: Option<NonZeroU32>,
    pub week_count: NonZeroU32,
    pub periodicity_cuts: BTreeSet<NonZeroU32>,
    pub costs_adjustments: CostsAdjustments,
}

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Week(u32);

impl Week {
    pub fn new(number: u32) -> Week {
        Week(number)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekPattern {
    pub name: String,
    pub weeks: BTreeSet<Week>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher {
    pub surname: String,
    pub firstname: String,
    pub contact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Student {
    pub surname: String,
    pub firstname: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub no_consecutive_slots: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectGroup {
    pub name: String,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotStart {
    pub day: crate::time::Weekday,
    pub time: crate::time::Time,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatSlot<WeekPatternId: OrdId> {
    pub week_pattern_id: WeekPatternId,
    pub start: SlotStart,
    pub duration: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncompatGroup<WeekPatternId: OrdId> {
    pub slots: BTreeSet<IncompatSlot<WeekPatternId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompat<WeekPatternId: OrdId> {
    pub name: String,
    pub max_count: usize,
    pub groups: BTreeSet<IncompatGroup<WeekPatternId>>,
}

impl<WeekPatternId: OrdId> Incompat<WeekPatternId> {
    pub fn references_week_pattern(&self, week_pattern_id: WeekPatternId) -> bool {
        for group in &self.groups {
            for slot in &group.slots {
                if slot.week_pattern_id == week_pattern_id {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub extendable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupList<StudentId: OrdId> {
    pub name: String,
    pub groups: Vec<Group>,
    pub students_mapping: BTreeMap<StudentId, usize>,
}

impl<StudentId: OrdId> GroupList<StudentId> {
    pub fn references_student(&self, student_id: StudentId) -> bool {
        self.students_mapping.contains_key(&student_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalancingConstraints {
    OptimizeOnly,
    OverallOnly,
    StrictWithCuts,
    StrictWithCutsAndOverall,
    Strict,
    OptimizeAndConsecutiveDifferentTeachers,
    OverallAndConsecutiveDifferentTeachers,
    StrictWithCutsAndConsecutiveDifferentTeachers,
    StrictWithCutsAndOverallAndConsecutiveDifferentTeachers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalancingSlotSelections {
    TeachersAndTimeSlots,
    Teachers,
    TimeSlots,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalancingRequirements {
    pub constraints: BalancingConstraints,
    pub slot_selections: BalancingSlotSelections,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject<SubjectGroupId: OrdId, IncompatId: OrdId, GroupListId: OrdId> {
    pub name: String,
    pub subject_group_id: SubjectGroupId,
    pub incompat_id: Option<IncompatId>,
    pub group_list_id: Option<GroupListId>,
    pub duration: NonZeroU32,
    pub students_per_group: RangeInclusive<NonZeroUsize>,
    pub period: NonZeroU32,
    pub period_is_strict: bool,
    pub is_tutorial: bool,
    pub max_groups_per_slot: NonZeroUsize,
    pub balancing_requirements: BalancingRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSlot<SubjectId: OrdId, TeacherId: OrdId, WeekPatternId: OrdId> {
    pub subject_id: SubjectId,
    pub teacher_id: TeacherId,
    pub start: SlotStart,
    pub week_pattern_id: WeekPatternId,
    pub room: String,
    pub cost: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotGroup<TimeSlotId: OrdId> {
    pub slots: BTreeSet<TimeSlotId>,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSelection<SubjectId: OrdId, TimeSlotId: OrdId> {
    pub subject_id: SubjectId,
    pub slot_groups: Vec<SlotGroup<TimeSlotId>>,
}

impl<SubjectId: OrdId, TimeSlotId: OrdId> SlotSelection<SubjectId, TimeSlotId> {
    fn references_time_slot(&self, index: TimeSlotId) -> bool {
        for slot_group in &self.slot_groups {
            if slot_group.slots.contains(&index) {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grouping<TimeSlotId: OrdId> {
    pub name: String,
    pub slots: BTreeSet<TimeSlotId>,
}

impl<TimeSlotId: OrdId> Grouping<TimeSlotId> {
    pub fn references_time_slot(&self, time_slot_id: TimeSlotId) -> bool {
        self.slots.contains(&time_slot_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupingIncompat<GroupingId: OrdId> {
    pub max_count: NonZeroUsize,
    pub groupings: BTreeSet<GroupingId>,
}

impl<GroupingId: OrdId> GroupingIncompat<GroupingId> {
    pub fn references_grouping(&self, grouping_id: GroupingId) -> bool {
        self.groupings.contains(&grouping_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeTimeSlot<TeacherId: OrdId> {
    pub teacher_id: TeacherId,
    pub start: SlotStart,
    pub room: String,
    pub group_assignments: BTreeMap<Week, BTreeSet<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupList<StudentId: OrdId> {
    pub name: String,
    pub groups: Vec<String>,
    pub students_mapping: BTreeMap<StudentId, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeSubject<TeacherId: OrdId, StudentId: OrdId> {
    pub time_slots: Vec<ColloscopeTimeSlot<TeacherId>>,
    pub group_list: ColloscopeGroupList<StudentId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope<TeacherId: OrdId, SubjectId: OrdId, StudentId: OrdId> {
    pub name: String,
    pub subjects: BTreeMap<SubjectId, ColloscopeSubject<TeacherId, StudentId>>,
}

impl<TeacherId: OrdId, SubjectId: OrdId, StudentId: OrdId>
    Colloscope<TeacherId, SubjectId, StudentId>
{
    pub fn references_teacher(&self, teacher_id: TeacherId) -> bool {
        for (_subject_id, subject) in &self.subjects {
            for time_slot in &subject.time_slots {
                if time_slot.teacher_id == teacher_id {
                    return true;
                }
            }
        }
        false
    }

    pub fn references_subject(&self, subject_id: SubjectId) -> bool {
        for (&current_subject_id, _subject) in &self.subjects {
            if subject_id == current_subject_id {
                return true;
            }
        }
        false
    }

    pub fn references_student(&self, student_id: StudentId) -> bool {
        for (_subject_id, subject) in &self.subjects {
            for (&current_student_id, _mapping) in &subject.group_list.students_mapping {
                if current_student_id == student_id {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub enum WeekPatternDependancy<IncompatId: OrdId, TimeSlotId: OrdId> {
    Incompat(IncompatId),
    TimeSlot(TimeSlotId),
}

#[derive(Clone, Debug)]
pub enum SubjectGroupDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum IncompatDependancy<SubjectId: OrdId, StudentId: OrdId> {
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum SubjectDependancy<
    TimeSlotId: OrdId,
    StudentId: OrdId,
    ColloscopeId: OrdId,
    SlotSelectionId: OrdId,
> {
    TimeSlot(TimeSlotId),
    Student(StudentId),
    Colloscope(ColloscopeId),
    SlotSelection(SlotSelectionId),
}

#[derive(Clone, Debug)]
pub enum TimeSlotDependancy<GroupingId: OrdId, SlotSelectionId: OrdId> {
    Grouping(GroupingId),
    SlotSelection(SlotSelectionId),
}

#[derive(Clone, Debug)]
pub enum TeacherDependancy<TimeSlotId: OrdId, ColloscopeId: OrdId> {
    TimeSlot(TimeSlotId),
    Colloscope(ColloscopeId),
}

#[derive(Clone, Debug)]
pub enum StudentDependancy<GroupListId: OrdId, ColloscopeId: OrdId> {
    GroupList(GroupListId),
    Colloscope(ColloscopeId),
}

#[derive(Clone, Debug)]
pub enum ColloscopeDependancy<TeacherId: OrdId, SubjectId: OrdId, StudentId: OrdId> {
    Teacher(TeacherId),
    Subject(SubjectId),
    Student(StudentId),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithId<Id: OrdId> {
    Ok,
    BadCrossId(Id),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithId2<Id1: OrdId, Id2: OrdId> {
    Ok,
    BadCrossId1(Id1),
    BadCrossId2(Id2),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithId3<Id1: OrdId, Id2: OrdId, Id3: OrdId> {
    Ok,
    BadCrossId1(Id1),
    BadCrossId2(Id2),
    BadCrossId3(Id3),
}

#[derive(Clone, Debug)]
pub enum DataStatusWithIdAndInvalidState<Id: OrdId> {
    Ok,
    InvalidData,
    BadCrossId(Id),
}

#[derive(Debug, Error)]
pub enum WeekPatternError<T: std::fmt::Debug + std::error::Error> {
    #[error("Week pattern references a week number ({0}) which exceeds general_data.week_count")]
    WeekNumberTooBig(u32),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

#[derive(Debug, Error)]
pub enum WeekPatternIdError<T, Id>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
{
    #[error("Id {0:?} is invalid")]
    InvalidId(Id),
    #[error("Week pattern references a week number ({0}) which exceeds general_data.week_count")]
    WeekNumberTooBig(u32),
    #[error("Backend internal error: {0:?}")]
    InternalError(#[from] T),
}

impl<T, Id> WeekPatternIdError<T, Id>
where
    T: std::fmt::Debug + std::error::Error,
    Id: std::fmt::Debug,
{
    fn from_week_pattern_error(error: WeekPatternError<T>) -> Self {
        match error {
            WeekPatternError::WeekNumberTooBig(id) => WeekPatternIdError::WeekNumberTooBig(id),
            WeekPatternError::InternalError(int_err) => WeekPatternIdError::InternalError(int_err),
        }
    }
}

#[derive(Debug)]
pub struct Logic<T: Storage> {
    storage: T,
}

impl<T: Storage> Logic<T> {
    pub fn new(storage: T) -> Self {
        Logic { storage }
    }
}

impl<T: Storage> Logic<T> {
    pub async fn general_data_set(
        &mut self,
        general_data: &GeneralData,
    ) -> std::result::Result<(), CheckedError<T::InternalError, Vec<T::WeekPatternId>>> {
        let week_patterns = self.week_patterns_get_all().await?;

        let mut errors = vec![];
        for (&week_pattern_id, week_pattern) in &week_patterns {
            if let Some(last_week) = week_pattern.weeks.last() {
                if last_week.0 >= general_data.week_count.get() {
                    errors.push(week_pattern_id);
                }
            }
        }

        if !errors.is_empty() {
            return Err(CheckedError::CheckFailed(errors));
        }

        Ok(unsafe { self.storage.general_data_set_unchecked(general_data) }.await?)
    }
    pub async fn general_data_get(&self) -> std::result::Result<GeneralData, T::InternalError> {
        self.storage.general_data_get().await
    }

    pub async fn week_patterns_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::WeekPatternId, WeekPattern>, T::InternalError> {
        self.storage.week_patterns_get_all().await
    }
    pub async fn week_patterns_get(
        &self,
        index: T::WeekPatternId,
    ) -> std::result::Result<WeekPattern, IdError<T::InternalError, T::WeekPatternId>> {
        self.storage.week_patterns_get(index).await
    }
    pub async fn week_patterns_check_id(
        &self,
        index: T::WeekPatternId,
    ) -> std::result::Result<bool, T::InternalError> {
        let week_patterns = self.week_patterns_get_all().await?;

        Ok(week_patterns.contains_key(&index))
    }
    pub async fn week_patterns_check_data(
        &self,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), WeekPatternError<T::InternalError>> {
        let general_data = self.general_data_get().await?;

        if let Some(last_week) = pattern.weeks.last() {
            if last_week.0 >= general_data.week_count.get() {
                return Err(WeekPatternError::WeekNumberTooBig(last_week.0));
            }
        }

        Ok(())
    }
    pub async fn week_patterns_add(
        &mut self,
        pattern: &WeekPattern,
    ) -> std::result::Result<T::WeekPatternId, WeekPatternError<T::InternalError>> {
        self.week_patterns_check_data(pattern).await?;

        Ok(unsafe { self.storage.week_patterns_add_unchecked(pattern) }.await?)
    }
    pub async fn week_patterns_update(
        &mut self,
        index: T::WeekPatternId,
        pattern: &WeekPattern,
    ) -> std::result::Result<(), WeekPatternIdError<T::InternalError, T::WeekPatternId>> {
        if !self.week_patterns_check_id(index).await? {
            return Err(WeekPatternIdError::InvalidId(index));
        }
        self.week_patterns_check_data(pattern)
            .await
            .map_err(WeekPatternIdError::from_week_pattern_error)?;

        Ok(unsafe { self.storage.week_patterns_update_unchecked(index, pattern) }.await?)
    }
    pub async fn week_patterns_check_can_remove(
        &self,
        index: T::WeekPatternId,
    ) -> std::result::Result<
        Vec<WeekPatternDependancy<T::IncompatId, T::TimeSlotId>>,
        IdError<T::InternalError, T::WeekPatternId>,
    > {
        let week_patterns = self.week_patterns_get_all().await?;

        if !week_patterns.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let incompats = self.incompats_get_all().await?;
        for (incompat_id, incompat) in incompats {
            if incompat.references_week_pattern(index) {
                dependancies.push(WeekPatternDependancy::Incompat(incompat_id));
            }
        }

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.week_pattern_id == index {
                dependancies.push(WeekPatternDependancy::TimeSlot(time_slot_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn week_patterns_remove(
        &mut self,
        index: T::WeekPatternId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::WeekPatternId,
            Vec<WeekPatternDependancy<T::IncompatId, T::TimeSlotId>>,
        >,
    > {
        let dependancies = self
            .week_patterns_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.week_patterns_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn teachers_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::TeacherId, Teacher>, T::InternalError> {
        self.storage.teachers_get_all().await
    }
    pub async fn teachers_get(
        &self,
        index: T::TeacherId,
    ) -> std::result::Result<Teacher, IdError<T::InternalError, T::TeacherId>> {
        self.storage.teachers_get(index).await
    }
    pub async fn teachers_add(
        &mut self,
        teacher: &Teacher,
    ) -> std::result::Result<T::TeacherId, T::InternalError> {
        self.storage.teachers_add(teacher).await
    }
    pub async fn teachers_update(
        &mut self,
        index: T::TeacherId,
        teacher: &Teacher,
    ) -> std::result::Result<(), IdError<T::InternalError, T::TeacherId>> {
        self.storage.teachers_update(index, teacher).await
    }
    pub async fn teachers_check_can_remove(
        &self,
        index: T::TeacherId,
    ) -> std::result::Result<
        Vec<TeacherDependancy<T::TimeSlotId, T::ColloscopeId>>,
        IdError<T::InternalError, T::TeacherId>,
    > {
        let teachers = self.teachers_get_all().await?;

        if !teachers.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.teacher_id == index {
                dependancies.push(TeacherDependancy::TimeSlot(time_slot_id));
            }
        }

        let colloscopes = self.colloscopes_get_all().await?;
        for (colloscope_id, colloscope) in colloscopes {
            if colloscope.references_teacher(index) {
                dependancies.push(TeacherDependancy::Colloscope(colloscope_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn teachers_remove(
        &mut self,
        index: T::TeacherId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::TeacherId,
            Vec<TeacherDependancy<T::TimeSlotId, T::ColloscopeId>>,
        >,
    > {
        let dependancies = self
            .teachers_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.teachers_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn students_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::StudentId, Student>, T::InternalError> {
        self.storage.students_get_all().await
    }
    pub async fn students_get(
        &self,
        index: T::StudentId,
    ) -> std::result::Result<Student, IdError<T::InternalError, T::StudentId>> {
        self.storage.students_get(index).await
    }
    pub async fn students_add(
        &mut self,
        student: &Student,
    ) -> std::result::Result<T::StudentId, T::InternalError> {
        self.storage.students_add(student).await
    }
    pub async fn students_update(
        &mut self,
        index: T::StudentId,
        student: &Student,
    ) -> std::result::Result<(), IdError<T::InternalError, T::StudentId>> {
        self.storage.students_update(index, student).await
    }
    pub async fn students_check_can_remove(
        &self,
        index: T::StudentId,
    ) -> std::result::Result<
        Vec<StudentDependancy<T::GroupListId, T::ColloscopeId>>,
        IdError<T::InternalError, T::StudentId>,
    > {
        let students = self.students_get_all().await?;

        if !students.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let group_lists = self.group_lists_get_all().await?;
        for (group_list_id, group_list) in group_lists {
            if group_list.references_student(index) {
                dependancies.push(StudentDependancy::GroupList(group_list_id));
            }
        }

        let colloscopes = self.colloscopes_get_all().await?;
        for (colloscope_id, colloscope) in colloscopes {
            if colloscope.references_student(index) {
                dependancies.push(StudentDependancy::Colloscope(colloscope_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn students_remove(
        &mut self,
        index: T::StudentId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::StudentId,
            Vec<StudentDependancy<T::GroupListId, T::ColloscopeId>>,
        >,
    > {
        let dependancies = self
            .students_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.students_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subject_groups_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::SubjectGroupId, SubjectGroup>, T::InternalError> {
        self.storage.subject_groups_get_all().await
    }
    pub async fn subject_groups_get(
        &self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<SubjectGroup, IdError<T::InternalError, T::SubjectGroupId>> {
        self.storage.subject_groups_get(index).await
    }
    pub async fn subject_groups_add(
        &mut self,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<T::SubjectGroupId, T::InternalError> {
        self.storage.subject_groups_add(subject_group).await
    }
    pub async fn subject_groups_update(
        &mut self,
        index: T::SubjectGroupId,
        subject_group: &SubjectGroup,
    ) -> std::result::Result<(), IdError<T::InternalError, T::SubjectGroupId>> {
        self.storage
            .subject_groups_update(index, subject_group)
            .await
    }
    pub async fn subject_groups_check_can_remove(
        &self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<
        Vec<SubjectGroupDependancy<T::SubjectId, T::StudentId>>,
        IdError<T::InternalError, T::SubjectGroupId>,
    > {
        let subject_groups = self.subject_groups_get_all().await?;

        if !subject_groups.contains_key(&index) {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.subject_group_id == index {
                dependancies.push(SubjectGroupDependancy::Subject(subject_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let subject_for_student = self.subject_group_for_student_get(student_id, index)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                    }
                )?;
            if subject_for_student.is_some() {
                dependancies.push(SubjectGroupDependancy::Student(student_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn subject_groups_remove(
        &mut self,
        index: T::SubjectGroupId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::SubjectGroupId,
            Vec<SubjectGroupDependancy<T::SubjectId, T::StudentId>>,
        >,
    > {
        let dependancies = self
            .subject_groups_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.subject_groups_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn incompats_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::IncompatId, Incompat<T::WeekPatternId>>, T::InternalError>
    {
        self.storage.incompats_get_all().await
    }
    pub async fn incompats_get(
        &self,
        index: T::IncompatId,
    ) -> std::result::Result<Incompat<T::WeekPatternId>, IdError<T::InternalError, T::IncompatId>>
    {
        self.storage.incompats_get(index).await
    }
    pub async fn incompats_check_id(
        &self,
        index: T::IncompatId,
    ) -> std::result::Result<bool, T::InternalError> {
        let incompats = self.incompats_get_all().await?;

        Ok(incompats.contains_key(&index))
    }
    pub async fn incompats_check_data(
        &self,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<DataStatusWithId<T::WeekPatternId>, T::InternalError> {
        let week_patterns = self.week_patterns_get_all().await?;

        for incompat_group in &incompat.groups {
            for incompat_slot in &incompat_group.slots {
                if !week_patterns.contains_key(&incompat_slot.week_pattern_id) {
                    return Ok(DataStatusWithId::BadCrossId(incompat_slot.week_pattern_id));
                }
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn incompats_add(
        &mut self,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<T::IncompatId, CrossError<T::InternalError, T::WeekPatternId>> {
        let data_status = self.incompats_check_data(incompat).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe { self.storage.incompats_add_unchecked(incompat) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn incompats_update(
        &mut self,
        index: T::IncompatId,
        incompat: &Incompat<T::WeekPatternId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::IncompatId, T::WeekPatternId>>
    {
        if !self.incompats_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self.incompats_check_data(incompat).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe { self.storage.incompats_update_unchecked(index, incompat) }.await?;
                Ok(())
            }
        }
    }
    pub async fn incompats_check_can_remove(
        &self,
        index: T::IncompatId,
    ) -> std::result::Result<
        Vec<IncompatDependancy<T::SubjectId, T::StudentId>>,
        IdError<T::InternalError, T::IncompatId>,
    > {
        if !self.incompats_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.incompat_id == Some(index) {
                dependancies.push(IncompatDependancy::Subject(subject_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let incompat_for_student = self.incompat_for_student_get(student_id, index)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was tested valid a few instructions ago", id2),
                    }
                )?;
            if incompat_for_student {
                dependancies.push(IncompatDependancy::Student(student_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn incompats_remove(
        &mut self,
        index: T::IncompatId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::IncompatId,
            Vec<IncompatDependancy<T::SubjectId, T::StudentId>>,
        >,
    > {
        let dependancies = self
            .incompats_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.incompats_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn group_lists_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::GroupListId, GroupList<T::StudentId>>, T::InternalError>
    {
        self.storage.group_lists_get_all().await
    }
    pub async fn group_lists_get(
        &self,
        index: T::GroupListId,
    ) -> std::result::Result<GroupList<T::StudentId>, IdError<T::InternalError, T::GroupListId>>
    {
        self.storage.group_lists_get(index).await
    }
    pub async fn group_lists_check_id(
        &self,
        index: T::GroupListId,
    ) -> std::result::Result<bool, T::InternalError> {
        let group_lists = self.group_lists_get_all().await?;

        Ok(group_lists.contains_key(&index))
    }
    pub async fn group_lists_check_data(
        &self,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<DataStatusWithIdAndInvalidState<T::StudentId>, T::InternalError> {
        let students = self.students_get_all().await?;

        for (&student_id, &group) in &group_list.students_mapping {
            if !students.contains_key(&student_id) {
                return Ok(DataStatusWithIdAndInvalidState::BadCrossId(student_id));
            }

            if group >= group_list.groups.len() {
                return Ok(DataStatusWithIdAndInvalidState::InvalidData);
            }
        }

        Ok(DataStatusWithIdAndInvalidState::Ok)
    }
    pub async fn group_lists_add(
        &mut self,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<
        T::GroupListId,
        InvalidCrossError<T::InternalError, GroupList<T::StudentId>, T::StudentId>,
    > {
        let data_status = self.group_lists_check_data(group_list).await?;
        match data_status {
            DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                Err(InvalidCrossError::InvalidCrossId(id))
            }
            DataStatusWithIdAndInvalidState::InvalidData => {
                Err(InvalidCrossError::InvalidData(group_list.clone()))
            }
            DataStatusWithIdAndInvalidState::Ok => {
                let id = unsafe { self.storage.group_lists_add_unchecked(group_list) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn group_lists_update(
        &mut self,
        index: T::GroupListId,
        group_list: &GroupList<T::StudentId>,
    ) -> std::result::Result<
        (),
        InvalidCrossIdError<
            T::InternalError,
            GroupList<T::StudentId>,
            T::GroupListId,
            T::StudentId,
        >,
    > {
        if !self.group_lists_check_id(index).await? {
            return Err(InvalidCrossIdError::InvalidId(index));
        }

        let data_status = self.group_lists_check_data(group_list).await?;
        match data_status {
            DataStatusWithIdAndInvalidState::BadCrossId(id) => {
                Err(InvalidCrossIdError::InvalidCrossId(id))
            }
            DataStatusWithIdAndInvalidState::InvalidData => {
                Err(InvalidCrossIdError::InvalidData(group_list.clone()))
            }
            DataStatusWithIdAndInvalidState::Ok => {
                unsafe { self.storage.group_lists_update_unchecked(index, group_list) }.await?;
                Ok(())
            }
        }
    }
    pub async fn group_lists_check_can_remove(
        &self,
        index: T::GroupListId,
    ) -> std::result::Result<Vec<T::SubjectId>, IdError<T::InternalError, T::GroupListId>> {
        if !self.group_lists_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let subjects = self.subjects_get_all().await?;
        for (subject_id, subject) in subjects {
            if subject.group_list_id == Some(index) {
                dependancies.push(subject_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn group_lists_remove(
        &mut self,
        index: T::GroupListId,
    ) -> std::result::Result<(), CheckedIdError<T::InternalError, T::GroupListId, Vec<T::SubjectId>>>
    {
        let dependancies = self
            .group_lists_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.group_lists_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subjects_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::SubjectId, Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>>,
        T::InternalError,
    > {
        self.storage.subjects_get_all().await
    }
    pub async fn subjects_get(
        &self,
        index: T::SubjectId,
    ) -> std::result::Result<
        Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        IdError<T::InternalError, T::SubjectId>,
    > {
        self.storage.subjects_get(index).await
    }
    pub async fn subjects_check_id(
        &self,
        index: T::SubjectId,
    ) -> std::result::Result<bool, T::InternalError> {
        let subjects = self.subjects_get_all().await?;

        Ok(subjects.contains_key(&index))
    }
    pub async fn subjects_check_data(
        &self,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        DataStatusWithId3<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
        T::InternalError,
    > {
        let subject_groups = self.subject_groups_get_all().await?;
        if !subject_groups.contains_key(&subject.subject_group_id) {
            return Ok(DataStatusWithId3::BadCrossId1(subject.subject_group_id));
        }

        if let Some(incompat_id) = subject.incompat_id {
            let incompats = self.incompats_get_all().await?;
            if !incompats.contains_key(&incompat_id) {
                return Ok(DataStatusWithId3::BadCrossId2(incompat_id));
            }
        }

        if let Some(group_list_id) = subject.group_list_id {
            let group_lists = self.group_lists_get_all().await?;
            if !group_lists.contains_key(&group_list_id) {
                return Ok(DataStatusWithId3::BadCrossId3(group_list_id));
            }
        }

        Ok(DataStatusWithId3::Ok)
    }
    pub async fn subjects_add(
        &mut self,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        T::SubjectId,
        Cross3Error<T::InternalError, T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    > {
        let data_status = self.subjects_check_data(subject).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                let id = unsafe { self.storage.subjects_add_unchecked(subject) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn subjects_update(
        &mut self,
        index: T::SubjectId,
        subject: &Subject<T::SubjectGroupId, T::IncompatId, T::GroupListId>,
    ) -> std::result::Result<
        (),
        Cross3IdWithDepError<
            T::InternalError,
            T::SubjectId,
            T::SubjectGroupId,
            T::IncompatId,
            T::GroupListId,
            T::StudentId,
        >,
    > {
        if !self.subjects_check_id(index).await? {
            return Err(Cross3IdWithDepError::InvalidId(index));
        }

        let current_subject = self.subjects_get(index).await.map_err(|e| match e {
            IdError::InvalidId(id) => panic!(
                "Subject group id {:?} should be valid as it was already checked",
                id
            ),
            IdError::InternalError(int_err) => Cross3IdWithDepError::InternalError(int_err),
        })?;

        if current_subject.subject_group_id != subject.subject_group_id {
            let students = self.students_get_all().await?;
            for (student_id, _student) in students {
                let subject_group_id = subject.subject_group_id;
                let subject_group_for_student = self.subject_group_for_student_get(student_id, subject_group_id)
                    .await
                    .map_err(
                        |e| match e {
                            Id2Error::InternalError(int_err) => Cross3IdWithDepError::InternalError(int_err),
                            Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                            Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was already checked", id2),
                        }
                    )?;
                if subject_group_for_student == Some(index) {
                    return Err(Cross3IdWithDepError::BlockingDependancy(student_id));
                }
            }
        }

        let data_status = self.subjects_check_data(subject).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdWithDepError::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdWithDepError::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdWithDepError::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                unsafe { self.storage.subjects_update_unchecked(index, subject) }.await?;
                Ok(())
            }
        }
    }
    pub async fn subjects_check_can_remove(
        &self,
        index: T::SubjectId,
    ) -> std::result::Result<
        Vec<SubjectDependancy<T::TimeSlotId, T::StudentId, T::ColloscopeId, T::SlotSelectionId>>,
        IdError<T::InternalError, T::SubjectId>,
    > {
        let subject = self.subjects_get(index).await?;

        let mut dependancies = Vec::new();

        let time_slots = self.time_slots_get_all().await?;
        for (time_slot_id, time_slot) in time_slots {
            if time_slot.subject_id == index {
                dependancies.push(SubjectDependancy::TimeSlot(time_slot_id));
            }
        }

        let students = self.students_get_all().await?;
        for (student_id, _student) in students {
            let subject_group_id = subject.subject_group_id;
            let subject_group_for_student = self.subject_group_for_student_get(student_id, subject_group_id)
                .await
                .map_err(
                    |e| match e {
                        Id2Error::InternalError(int_err) => IdError::InternalError(int_err),
                        Id2Error::InvalidId1(id1) => panic!("Student id {:?} should be valid as it was returned from students_get_all", id1),
                        Id2Error::InvalidId2(id2) => panic!("Subject group id {:?} should be valid as it was retrieved from the database", id2),
                    }
                )?;
            if subject_group_for_student == Some(index) {
                dependancies.push(SubjectDependancy::Student(student_id));
            }
        }

        let colloscopes = self.colloscopes_get_all().await?;
        for (colloscope_id, colloscope) in colloscopes {
            if colloscope.references_subject(index) {
                dependancies.push(SubjectDependancy::Colloscope(colloscope_id));
            }
        }

        let slot_selections = self.slot_selections_get_all().await?;
        for (slot_selection_id, slot_selection) in slot_selections {
            if slot_selection.subject_id == index {
                dependancies.push(SubjectDependancy::SlotSelection(slot_selection_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn subjects_remove(
        &mut self,
        index: T::SubjectId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::SubjectId,
            Vec<
                SubjectDependancy<T::TimeSlotId, T::StudentId, T::ColloscopeId, T::SlotSelectionId>,
            >,
        >,
    > {
        let dependancies = self
            .subjects_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.subjects_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn time_slots_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::TimeSlotId, TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>>,
        T::InternalError,
    > {
        self.storage.time_slots_get_all().await
    }
    pub async fn time_slots_get(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<
        TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        IdError<T::InternalError, T::TimeSlotId>,
    > {
        self.storage.time_slots_get(index).await
    }
    pub async fn time_slots_check_id(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<bool, T::InternalError> {
        let time_slots = self.time_slots_get_all().await?;

        Ok(time_slots.contains_key(&index))
    }
    pub async fn time_slots_check_data(
        &self,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        DataStatusWithId3<T::SubjectId, T::TeacherId, T::WeekPatternId>,
        T::InternalError,
    > {
        let subjects = self.subjects_get_all().await?;
        if !subjects.contains_key(&time_slot.subject_id) {
            return Ok(DataStatusWithId3::BadCrossId1(time_slot.subject_id));
        }

        let teachers = self.teachers_get_all().await?;
        if !teachers.contains_key(&time_slot.teacher_id) {
            return Ok(DataStatusWithId3::BadCrossId2(time_slot.teacher_id));
        }

        let week_patterns = self.week_patterns_get_all().await?;
        if !week_patterns.contains_key(&time_slot.week_pattern_id) {
            return Ok(DataStatusWithId3::BadCrossId3(time_slot.week_pattern_id));
        }

        Ok(DataStatusWithId3::Ok)
    }
    pub async fn time_slots_add(
        &mut self,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        T::TimeSlotId,
        Cross3Error<T::InternalError, T::SubjectId, T::TeacherId, T::WeekPatternId>,
    > {
        let data_status = self.time_slots_check_data(time_slot).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3Error::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3Error::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3Error::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                let id = unsafe { self.storage.time_slots_add_unchecked(time_slot) }.await?;
                Ok(id)
            }
        }
    }
    async fn is_time_slot_referenced_by_slot_selections(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<Option<T::SlotSelectionId>, T::InternalError> {
        let slot_selections = self.slot_selections_get_all().await?;

        for (slot_selection_id, slot_selection) in slot_selections {
            if slot_selection.references_time_slot(index) {
                return Ok(Some(slot_selection_id));
            }
        }

        Ok(None)
    }
    pub async fn time_slots_update(
        &mut self,
        index: T::TimeSlotId,
        time_slot: &TimeSlot<T::SubjectId, T::TeacherId, T::WeekPatternId>,
    ) -> std::result::Result<
        (),
        Cross3IdWithDepError<
            T::InternalError,
            T::TimeSlotId,
            T::SubjectId,
            T::TeacherId,
            T::WeekPatternId,
            T::SlotSelectionId,
        >,
    > {
        if !self.time_slots_check_id(index).await? {
            return Err(Cross3IdWithDepError::InvalidId(index));
        }

        if let Some(slot_selection_id) = self
            .is_time_slot_referenced_by_slot_selections(index)
            .await?
        {
            let current_time_slot = self.time_slots_get(index).await.map_err(|e| match e {
                IdError::InvalidId(id) => panic!(
                    "Time slot id {:?} should be valid as it was already checked",
                    id
                ),
                IdError::InternalError(int_err) => Cross3IdWithDepError::InternalError(int_err),
            })?;

            if current_time_slot.subject_id != time_slot.subject_id {
                return Err(Cross3IdWithDepError::BlockingDependancy(slot_selection_id));
            }
        }

        let data_status = self.time_slots_check_data(time_slot).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id1) => Err(Cross3IdWithDepError::InvalidCrossId1(id1)),
            DataStatusWithId3::BadCrossId2(id2) => Err(Cross3IdWithDepError::InvalidCrossId2(id2)),
            DataStatusWithId3::BadCrossId3(id3) => Err(Cross3IdWithDepError::InvalidCrossId3(id3)),
            DataStatusWithId3::Ok => {
                unsafe { self.storage.time_slots_update_unchecked(index, time_slot) }.await?;
                Ok(())
            }
        }
    }
    pub async fn time_slots_check_can_remove(
        &self,
        index: T::TimeSlotId,
    ) -> std::result::Result<
        Vec<TimeSlotDependancy<T::GroupingId, T::SlotSelectionId>>,
        IdError<T::InternalError, T::TimeSlotId>,
    > {
        if !self.time_slots_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let groupings = self.groupings_get_all().await?;
        for (grouping_id, grouping) in groupings {
            if grouping.references_time_slot(index) {
                dependancies.push(TimeSlotDependancy::Grouping(grouping_id));
            }
        }

        let slot_selections = self.slot_selections_get_all().await?;
        for (slot_selection_id, slot_selection) in slot_selections {
            if slot_selection.references_time_slot(index) {
                dependancies.push(TimeSlotDependancy::SlotSelection(slot_selection_id));
            }
        }

        Ok(dependancies)
    }
    pub async fn time_slots_remove(
        &mut self,
        index: T::TimeSlotId,
    ) -> std::result::Result<
        (),
        CheckedIdError<
            T::InternalError,
            T::TimeSlotId,
            Vec<TimeSlotDependancy<T::GroupingId, T::SlotSelectionId>>,
        >,
    > {
        let dependancies = self
            .time_slots_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.time_slots_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn groupings_get_all(
        &self,
    ) -> std::result::Result<BTreeMap<T::GroupingId, Grouping<T::TimeSlotId>>, T::InternalError>
    {
        self.storage.groupings_get_all().await
    }
    pub async fn groupings_get(
        &self,
        index: T::GroupingId,
    ) -> std::result::Result<Grouping<T::TimeSlotId>, IdError<T::InternalError, T::GroupingId>>
    {
        self.storage.groupings_get(index).await
    }
    pub async fn groupings_check_id(
        &self,
        index: T::GroupingId,
    ) -> std::result::Result<bool, T::InternalError> {
        let groupings = self.groupings_get_all().await?;

        Ok(groupings.contains_key(&index))
    }
    pub async fn groupings_check_data(
        &self,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<DataStatusWithId<T::TimeSlotId>, T::InternalError> {
        let time_slots = self.time_slots_get_all().await?;
        for &slot_id in &grouping.slots {
            if !time_slots.contains_key(&slot_id) {
                return Ok(DataStatusWithId::BadCrossId(slot_id));
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn groupings_add(
        &mut self,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<T::GroupingId, CrossError<T::InternalError, T::TimeSlotId>> {
        let data_status = self.groupings_check_data(grouping).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe { self.storage.groupings_add_unchecked(grouping) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn groupings_update(
        &mut self,
        index: T::GroupingId,
        grouping: &Grouping<T::TimeSlotId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::GroupingId, T::TimeSlotId>> {
        if !self.groupings_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self.groupings_check_data(grouping).await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe { self.storage.groupings_update_unchecked(index, grouping) }.await?;
                Ok(())
            }
        }
    }
    pub async fn groupings_check_can_remove(
        &self,
        index: T::GroupingId,
    ) -> std::result::Result<Vec<T::GroupingIncompatId>, IdError<T::InternalError, T::GroupingId>>
    {
        if !self.groupings_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        let mut dependancies = Vec::new();

        let grouping_incompats = self.grouping_incompats_get_all().await?;
        for (grouping_incompat_id, grouping_incompat) in grouping_incompats {
            if grouping_incompat.references_grouping(index) {
                dependancies.push(grouping_incompat_id);
            }
        }

        Ok(dependancies)
    }
    pub async fn groupings_remove(
        &mut self,
        index: T::GroupingId,
    ) -> std::result::Result<
        (),
        CheckedIdError<T::InternalError, T::GroupingId, Vec<T::GroupingIncompatId>>,
    > {
        let dependancies = self
            .groupings_check_can_remove(index)
            .await
            .map_err(CheckedIdError::from_id_error)?;
        if dependancies.len() != 0 {
            return Err(CheckedIdError::CheckFailed(dependancies));
        }
        unsafe { self.storage.groupings_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn grouping_incompats_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::GroupingIncompatId, GroupingIncompat<T::GroupingId>>,
        T::InternalError,
    > {
        self.storage.grouping_incompats_get_all().await
    }
    pub async fn grouping_incompats_get(
        &self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<
        GroupingIncompat<T::GroupingId>,
        IdError<T::InternalError, T::GroupingIncompatId>,
    > {
        self.storage.grouping_incompats_get(index).await
    }
    pub async fn grouping_incompats_check_id(
        &self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<bool, T::InternalError> {
        let grouping_incompats = self.grouping_incompats_get_all().await?;

        Ok(grouping_incompats.contains_key(&index))
    }
    pub async fn grouping_incompats_check_data(
        &self,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<DataStatusWithId<T::GroupingId>, T::InternalError> {
        let groupings = self.groupings_get_all().await?;

        for &grouping_id in &grouping_incompat.groupings {
            if !groupings.contains_key(&grouping_id) {
                return Ok(DataStatusWithId::BadCrossId(grouping_id));
            }
        }

        Ok(DataStatusWithId::Ok)
    }
    pub async fn grouping_incompats_add(
        &mut self,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<T::GroupingIncompatId, CrossError<T::InternalError, T::GroupingId>>
    {
        let data_status = self
            .grouping_incompats_check_data(grouping_incompat)
            .await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                let id = unsafe {
                    self.storage
                        .grouping_incompats_add_unchecked(grouping_incompat)
                }
                .await?;
                Ok(id)
            }
        }
    }
    pub async fn grouping_incompats_update(
        &mut self,
        index: T::GroupingIncompatId,
        grouping_incompat: &GroupingIncompat<T::GroupingId>,
    ) -> std::result::Result<(), CrossIdError<T::InternalError, T::GroupingIncompatId, T::GroupingId>>
    {
        if !self.grouping_incompats_check_id(index).await? {
            return Err(CrossIdError::InvalidId(index));
        }

        let data_status = self
            .grouping_incompats_check_data(grouping_incompat)
            .await?;
        match data_status {
            DataStatusWithId::BadCrossId(id) => Err(CrossIdError::InvalidCrossId(id)),
            DataStatusWithId::Ok => {
                unsafe {
                    self.storage
                        .grouping_incompats_update_unchecked(index, grouping_incompat)
                }
                .await?;
                Ok(())
            }
        }
    }
    pub async fn grouping_incompats_check_can_remove(
        &self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::GroupingIncompatId>> {
        if !self.grouping_incompats_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        Ok(())
    }
    pub async fn grouping_incompats_remove(
        &mut self,
        index: T::GroupingIncompatId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::GroupingIncompatId>> {
        self.grouping_incompats_check_can_remove(index).await?;

        unsafe { self.storage.grouping_incompats_remove_unchecked(index) }.await?;
        Ok(())
    }

    pub async fn subject_group_for_student_get(
        &self,
        student_id: T::StudentId,
        subject_group_id: T::SubjectGroupId,
    ) -> std::result::Result<
        Option<T::SubjectId>,
        Id2Error<T::InternalError, T::StudentId, T::SubjectGroupId>,
    > {
        self.storage
            .subject_group_for_student_get(student_id, subject_group_id)
            .await
    }
    pub async fn subject_group_for_student_set(
        &mut self,
        student_id: T::StudentId,
        subject_group_id: T::SubjectGroupId,
        subject_id: Option<T::SubjectId>,
    ) -> std::result::Result<
        (),
        CrossId3Error<
            T::InternalError,
            T::StudentId,
            T::SubjectGroupId,
            T::SubjectId,
            T::SubjectId,
        >,
    > {
        let students = self.students_get_all().await?;
        if !students.contains_key(&student_id) {
            return Err(CrossId3Error::InvalidId1(student_id));
        }
        let subject_groups = self.subject_groups_get_all().await?;
        if !subject_groups.contains_key(&subject_group_id) {
            return Err(CrossId3Error::InvalidId2(subject_group_id));
        }
        if let Some(id) = subject_id {
            let subject = self.subjects_get(id).await.map_err(|e| match e {
                IdError::InternalError(int_err) => CrossId3Error::InternalError(int_err),
                IdError::InvalidId(_) => CrossId3Error::InvalidId3(id),
            })?;
            if subject.subject_group_id != subject_group_id {
                return Err(CrossId3Error::InvalidCrossId(id));
            }
        }
        unsafe {
            self.storage.subject_group_for_student_set_unchecked(
                student_id,
                subject_group_id,
                subject_id,
            )
        }
        .await?;
        Ok(())
    }

    pub async fn incompat_for_student_get(
        &self,
        student_id: T::StudentId,
        incompat_id: T::IncompatId,
    ) -> std::result::Result<bool, Id2Error<T::InternalError, T::StudentId, T::IncompatId>> {
        self.storage
            .incompat_for_student_get(student_id, incompat_id)
            .await
    }
    pub async fn incompat_for_student_set(
        &mut self,
        student_id: T::StudentId,
        incompat_id: T::IncompatId,
        enabled: bool,
    ) -> std::result::Result<(), Id2Error<T::InternalError, T::StudentId, T::IncompatId>> {
        let students = self.students_get_all().await?;
        if !students.contains_key(&student_id) {
            return Err(Id2Error::InvalidId1(student_id));
        }
        if !self.incompats_check_id(incompat_id).await? {
            return Err(Id2Error::InvalidId2(incompat_id));
        }
        unsafe {
            self.storage
                .incompat_for_student_set_unchecked(student_id, incompat_id, enabled)
        }
        .await?;
        Ok(())
    }

    pub async fn colloscopes_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::ColloscopeId, Colloscope<T::TeacherId, T::SubjectId, T::StudentId>>,
        T::InternalError,
    > {
        self.storage.colloscopes_get_all().await
    }
    pub async fn colloscopes_get(
        &self,
        index: T::ColloscopeId,
    ) -> std::result::Result<
        Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
        IdError<T::InternalError, T::ColloscopeId>,
    > {
        self.storage.colloscopes_get(index).await
    }
    pub async fn colloscopes_check_id(
        &self,
        index: T::ColloscopeId,
    ) -> std::result::Result<bool, T::InternalError> {
        let colloscopes = self.colloscopes_get_all().await?;

        Ok(colloscopes.contains_key(&index))
    }
    pub async fn colloscopes_check_data(
        &self,
        colloscope: &Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
    ) -> std::result::Result<
        DataStatusWithId3<T::TeacherId, T::SubjectId, T::StudentId>,
        T::InternalError,
    > {
        let teachers = self.teachers_get_all().await?;
        let subjects = self.subjects_get_all().await?;
        let students = self.students_get_all().await?;

        for (&subject_id, subject_desc) in &colloscope.subjects {
            if !subjects.contains_key(&subject_id) {
                return Ok(DataStatusWithId3::BadCrossId2(subject_id));
            }

            for (&student_id, _mapping) in &subject_desc.group_list.students_mapping {
                if !students.contains_key(&student_id) {
                    return Ok(DataStatusWithId3::BadCrossId3(student_id));
                }
            }

            for time_slot in &subject_desc.time_slots {
                if !teachers.contains_key(&time_slot.teacher_id) {
                    return Ok(DataStatusWithId3::BadCrossId1(time_slot.teacher_id));
                }
            }
        }

        Ok(DataStatusWithId3::Ok)
    }
    pub async fn colloscopes_add(
        &mut self,
        colloscope: &Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
    ) -> std::result::Result<
        T::ColloscopeId,
        Cross3Error<T::InternalError, T::TeacherId, T::SubjectId, T::StudentId>,
    > {
        let data_status = self.colloscopes_check_data(colloscope).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id) => Err(Cross3Error::InvalidCrossId1(id)),
            DataStatusWithId3::BadCrossId2(id) => Err(Cross3Error::InvalidCrossId2(id)),
            DataStatusWithId3::BadCrossId3(id) => Err(Cross3Error::InvalidCrossId3(id)),
            DataStatusWithId3::Ok => {
                let id = unsafe { self.storage.colloscopes_add_unchecked(colloscope) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn colloscopes_check_can_remove(
        &self,
        index: T::ColloscopeId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::ColloscopeId>> {
        if !self.colloscopes_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        Ok(())
    }
    pub async fn colloscopes_remove(
        &mut self,
        index: T::ColloscopeId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::ColloscopeId>> {
        self.colloscopes_check_can_remove(index).await?;

        unsafe { self.storage.colloscopes_remove_unchecked(index) }.await?;
        Ok(())
    }
    pub async fn colloscopes_update(
        &mut self,
        index: T::ColloscopeId,
        colloscope: &Colloscope<T::TeacherId, T::SubjectId, T::StudentId>,
    ) -> std::result::Result<
        (),
        Cross3IdError<T::InternalError, T::ColloscopeId, T::TeacherId, T::SubjectId, T::StudentId>,
    > {
        if !self.colloscopes_check_id(index).await? {
            return Err(Cross3IdError::InvalidId(index));
        }

        let data_status = self.colloscopes_check_data(colloscope).await?;
        match data_status {
            DataStatusWithId3::BadCrossId1(id) => Err(Cross3IdError::InvalidCrossId1(id)),
            DataStatusWithId3::BadCrossId2(id) => Err(Cross3IdError::InvalidCrossId2(id)),
            DataStatusWithId3::BadCrossId3(id) => Err(Cross3IdError::InvalidCrossId3(id)),
            DataStatusWithId3::Ok => {
                unsafe { self.storage.colloscopes_update_unchecked(index, colloscope) }.await?;
                Ok(())
            }
        }
    }

    pub async fn slot_selections_get_all(
        &self,
    ) -> std::result::Result<
        BTreeMap<T::SlotSelectionId, SlotSelection<T::SubjectId, T::TimeSlotId>>,
        T::InternalError,
    > {
        self.storage.slot_selections_get_all().await
    }
    pub async fn slot_selections_get(
        &self,
        index: T::SlotSelectionId,
    ) -> std::result::Result<
        SlotSelection<T::SubjectId, T::TimeSlotId>,
        IdError<T::InternalError, T::SlotSelectionId>,
    > {
        self.storage.slot_selections_get(index).await
    }
    pub async fn slot_selections_check_id(
        &self,
        index: T::SlotSelectionId,
    ) -> std::result::Result<bool, T::InternalError> {
        let slot_selections = self.slot_selections_get_all().await?;

        Ok(slot_selections.contains_key(&index))
    }
    pub async fn slot_selections_check_data(
        &self,
        slot_selection: &SlotSelection<T::SubjectId, T::TimeSlotId>,
    ) -> std::result::Result<DataStatusWithId2<T::SubjectId, T::TimeSlotId>, T::InternalError> {
        let subjects = self.subjects_get_all().await?;
        let time_slots = self.time_slots_get_all().await?;

        if !subjects.contains_key(&slot_selection.subject_id) {
            return Ok(DataStatusWithId2::BadCrossId1(slot_selection.subject_id));
        }

        for slot_group in &slot_selection.slot_groups {
            for &time_slot_id in &slot_group.slots {
                match time_slots.get(&time_slot_id) {
                    None => return Ok(DataStatusWithId2::BadCrossId2(time_slot_id)),
                    Some(time_slot) => {
                        if time_slot.subject_id != slot_selection.subject_id {
                            return Ok(DataStatusWithId2::BadCrossId2(time_slot_id));
                        }
                    }
                }
            }
        }

        Ok(DataStatusWithId2::Ok)
    }
    pub async fn slot_selections_add(
        &mut self,
        slot_selection: &SlotSelection<T::SubjectId, T::TimeSlotId>,
    ) -> std::result::Result<
        T::SlotSelectionId,
        Cross2Error<T::InternalError, T::SubjectId, T::TimeSlotId>,
    > {
        let data_status = self.slot_selections_check_data(slot_selection).await?;
        match data_status {
            DataStatusWithId2::BadCrossId1(id) => Err(Cross2Error::InvalidCrossId1(id)),
            DataStatusWithId2::BadCrossId2(id) => Err(Cross2Error::InvalidCrossId2(id)),
            DataStatusWithId2::Ok => {
                let id =
                    unsafe { self.storage.slot_selections_add_unchecked(slot_selection) }.await?;
                Ok(id)
            }
        }
    }
    pub async fn slot_selections_check_can_remove(
        &self,
        index: T::SlotSelectionId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::SlotSelectionId>> {
        if !self.slot_selections_check_id(index).await? {
            return Err(IdError::InvalidId(index));
        }

        Ok(())
    }
    pub async fn slot_selections_remove(
        &mut self,
        index: T::SlotSelectionId,
    ) -> std::result::Result<(), IdError<T::InternalError, T::SlotSelectionId>> {
        self.slot_selections_check_can_remove(index).await?;

        unsafe { self.storage.slot_selections_remove_unchecked(index) }.await?;
        Ok(())
    }
    pub async fn slot_selections_update(
        &mut self,
        index: T::SlotSelectionId,
        slot_selection: &SlotSelection<T::SubjectId, T::TimeSlotId>,
    ) -> std::result::Result<
        (),
        Cross2IdError<T::InternalError, T::SlotSelectionId, T::SubjectId, T::TimeSlotId>,
    > {
        if !self.slot_selections_check_id(index).await? {
            return Err(Cross2IdError::InvalidId(index));
        }

        let data_status = self.slot_selections_check_data(slot_selection).await?;
        match data_status {
            DataStatusWithId2::BadCrossId1(id) => Err(Cross2IdError::InvalidCrossId1(id)),
            DataStatusWithId2::BadCrossId2(id) => Err(Cross2IdError::InvalidCrossId2(id)),
            DataStatusWithId2::Ok => {
                unsafe {
                    self.storage
                        .slot_selections_update_unchecked(index, slot_selection)
                }
                .await?;
                Ok(())
            }
        }
    }

    pub async fn backup(&self) -> BackupResult<BackupData, T::InternalError> {
        let mut next_id = 0;
        let general_data = self.general_data_get().await?;

        let mut week_pattern_ids = BTreeMap::<T::WeekPatternId, BackupWeekPatternId>::new();
        let week_patterns: BTreeMap<_, _> = self
            .week_patterns_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut week_pattern_ids, id);
                (new_id, x)
            })
            .collect();

        let mut teacher_ids = BTreeMap::<T::TeacherId, BackupTeacherId>::new();
        let teachers: BTreeMap<_, _> = self
            .teachers_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut teacher_ids, id);
                (new_id, x)
            })
            .collect();

        let mut student_ids = BTreeMap::<T::StudentId, BackupStudentId>::new();
        let students: BTreeMap<_, _> = self
            .students_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut student_ids, id);
                (new_id, x)
            })
            .collect();

        let mut subject_group_ids = BTreeMap::<T::SubjectGroupId, BackupSubjectGroupId>::new();
        let subject_groups: BTreeMap<_, _> = self
            .subject_groups_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut subject_group_ids, id);
                (new_id, x)
            })
            .collect();

        let mut incompat_ids = BTreeMap::<T::IncompatId, BackupIncompatId>::new();
        let incompats: BTreeMap<_, _> = self
            .incompats_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut incompat_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_incompat(x, &week_pattern_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut group_list_ids = BTreeMap::<T::GroupListId, BackupGroupListId>::new();
        let group_lists: BTreeMap<_, _> = self
            .group_lists_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut group_list_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_group_list(x, &student_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut subject_ids = BTreeMap::<T::SubjectId, BackupSubjectId>::new();
        let subjects: BTreeMap<_, _> = self
            .subjects_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut subject_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_subject(x, &subject_group_ids, &incompat_ids, &group_list_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut time_slot_ids = BTreeMap::<T::TimeSlotId, BackupTimeSlotId>::new();
        let time_slots: BTreeMap<_, _> = self
            .time_slots_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut time_slot_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_time_slot(x, &subject_ids, &teacher_ids, &week_pattern_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut grouping_ids = BTreeMap::<T::GroupingId, BackupGroupingId>::new();
        let groupings: BTreeMap<_, _> = self
            .groupings_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut grouping_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping(x, &time_slot_ids).ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut grouping_incompat_ids =
            BTreeMap::<T::GroupingIncompatId, BackupGroupingIncompatId>::new();
        let grouping_incompats: BTreeMap<_, _> = self
            .grouping_incompats_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut grouping_incompat_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_grouping_incompat(x, &grouping_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut colloscope_ids = BTreeMap::<T::ColloscopeId, BackupColloscopeId>::new();
        let colloscopes: BTreeMap<_, _> = self
            .colloscopes_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut colloscope_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_colloscope(x, &teacher_ids, &subject_ids, &student_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut slot_selection_ids = BTreeMap::<T::SlotSelectionId, BackupSlotSelectionId>::new();
        let slot_selections: BTreeMap<_, _> = self
            .slot_selections_get_all()
            .await?
            .into_iter()
            .map(|(id, x)| {
                let new_id = get_backup_id(&mut next_id, &mut slot_selection_ids, id);
                BackupResult::<_, T::InternalError>::Ok((
                    new_id,
                    translate_slot_selection(x, &subject_ids, &time_slot_ids)
                        .ok_or(BackupError::InconsistentId)?,
                ))
            })
            .collect::<BackupResult<_, _>>()?;

        let mut student_assignments = BTreeMap::<_, _>::new();
        for (old_student_id, new_student_id) in &student_ids {
            let mut incompats = BTreeSet::<BackupIncompatId>::new();
            for (old_incompat_id, new_incompat_id) in &incompat_ids {
                if self
                    .incompat_for_student_get(*old_student_id, *old_incompat_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => BackupError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => BackupError::InconsistentId,
                        Id2Error::InvalidId2(_) => BackupError::InconsistentId,
                    })?
                {
                    incompats.insert(*new_incompat_id);
                }
            }

            let mut subject_groups = BTreeMap::<BackupSubjectGroupId, BackupSubjectId>::new();
            for (old_subject_group_id, new_subject_group_id) in &subject_group_ids {
                let old_subject_id_opt = self
                    .subject_group_for_student_get(*old_student_id, *old_subject_group_id)
                    .await
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => BackupError::InternalError(int_err),
                        Id2Error::InvalidId1(_) => BackupError::InconsistentId,
                        Id2Error::InvalidId2(_) => BackupError::InconsistentId,
                    })?;

                if let Some(old_subject_id) = old_subject_id_opt {
                    let new_subject_id = subject_ids
                        .get(&old_subject_id)
                        .cloned()
                        .ok_or(BackupError::InconsistentId)?;
                    subject_groups.insert(*new_subject_group_id, new_subject_id);
                }
            }

            student_assignments.insert(
                *new_student_id,
                BackupAssignments {
                    subject_groups,
                    incompats,
                },
            );
        }

        Ok(BackupData {
            next_id,
            general_data,
            week_patterns,
            teachers,
            students,
            subject_groups,
            incompats,
            group_lists,
            subjects,
            time_slots,
            groupings,
            grouping_incompats,
            colloscopes,
            slot_selections,
            student_assignments,
        })
    }
}

#[derive(Debug, Error)]
pub enum BackupError<T: std::fmt::Debug + std::error::Error> {
    #[error("Error while retrieving data from backend")]
    InternalError(#[from] T),
    #[error("Data has inconstent ids")]
    InconsistentId,
}

pub type BackupResult<T, E> = std::result::Result<T, BackupError<E>>;

fn get_backup_id<T: OrdId, U: OrdId + From<u64>>(
    next_id: &mut u64,
    id_map: &mut BTreeMap<T, U>,
    id: T,
) -> U {
    match id_map.get(&id) {
        Some(new_id) => new_id.clone(),
        None => {
            let new_id = U::from(*next_id);
            *next_id += 1;
            id_map.insert(id, new_id.clone());
            new_id
        }
    }
}

fn translate_incompat<T: OrdId>(
    incompat: Incompat<T>,
    week_pattern_ids: &BTreeMap<T, BackupWeekPatternId>,
) -> Option<Incompat<BackupWeekPatternId>> {
    Some(Incompat {
        name: incompat.name,
        max_count: incompat.max_count,
        groups: incompat
            .groups
            .into_iter()
            .map(|x| {
                Some(IncompatGroup::<BackupWeekPatternId> {
                    slots: x
                        .slots
                        .into_iter()
                        .map(|y| {
                            Some(IncompatSlot::<BackupWeekPatternId> {
                                start: y.start,
                                duration: y.duration,
                                week_pattern_id: week_pattern_ids
                                    .get(&y.week_pattern_id)
                                    .cloned()?,
                            })
                        })
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_group_list<T: OrdId>(
    group_list: GroupList<T>,
    student_ids: &BTreeMap<T, BackupStudentId>,
) -> Option<GroupList<BackupStudentId>> {
    Some(GroupList {
        name: group_list.name,
        groups: group_list.groups,
        students_mapping: group_list
            .students_mapping
            .into_iter()
            .map(|(student_id, group)| {
                let new_student_id = student_ids.get(&student_id).cloned()?;

                Some((new_student_id, group))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_subject<T: OrdId, U: OrdId, V: OrdId>(
    subject: Subject<T, U, V>,
    subject_group_ids: &BTreeMap<T, BackupSubjectGroupId>,
    incompat_ids: &BTreeMap<U, BackupIncompatId>,
    group_list_ids: &BTreeMap<V, BackupGroupListId>,
) -> Option<Subject<BackupSubjectGroupId, BackupIncompatId, BackupGroupListId>> {
    Some(Subject {
        name: subject.name,
        subject_group_id: subject_group_ids.get(&subject.subject_group_id).cloned()?,
        incompat_id: match subject.incompat_id {
            Some(old_id) => Some(incompat_ids.get(&old_id).cloned()?),
            None => None,
        },
        group_list_id: match subject.group_list_id {
            Some(old_id) => Some(group_list_ids.get(&old_id).cloned()?),
            None => None,
        },
        duration: subject.duration,
        students_per_group: subject.students_per_group,
        period: subject.period,
        period_is_strict: subject.period_is_strict,
        is_tutorial: subject.is_tutorial,
        max_groups_per_slot: subject.max_groups_per_slot,
        balancing_requirements: subject.balancing_requirements,
    })
}

fn translate_time_slot<T: OrdId, U: OrdId, V: OrdId>(
    time_slot: TimeSlot<T, U, V>,
    subject_ids: &BTreeMap<T, BackupSubjectId>,
    teacher_ids: &BTreeMap<U, BackupTeacherId>,
    week_pattern_ids: &BTreeMap<V, BackupWeekPatternId>,
) -> Option<TimeSlot<BackupSubjectId, BackupTeacherId, BackupWeekPatternId>> {
    Some(TimeSlot {
        subject_id: subject_ids.get(&time_slot.subject_id).cloned()?,
        teacher_id: teacher_ids.get(&time_slot.teacher_id).cloned()?,
        start: time_slot.start,
        week_pattern_id: week_pattern_ids.get(&time_slot.week_pattern_id).cloned()?,
        room: time_slot.room,
        cost: time_slot.cost,
    })
}

fn translate_grouping<T: OrdId>(
    grouping: Grouping<T>,
    time_slot_ids: &BTreeMap<T, BackupTimeSlotId>,
) -> Option<Grouping<BackupTimeSlotId>> {
    Some(Grouping {
        name: grouping.name,
        slots: grouping
            .slots
            .into_iter()
            .map(|time_slot_id| {
                let new_time_slot_id = time_slot_ids.get(&time_slot_id).cloned()?;

                Some(new_time_slot_id)
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_grouping_incompat<T: OrdId>(
    grouping_incompat: GroupingIncompat<T>,
    grouping_ids: &BTreeMap<T, BackupGroupingId>,
) -> Option<GroupingIncompat<BackupGroupingId>> {
    Some(GroupingIncompat {
        max_count: grouping_incompat.max_count,
        groupings: grouping_incompat
            .groupings
            .into_iter()
            .map(|grouping_id| {
                let new_grouping_id = grouping_ids.get(&grouping_id).cloned()?;

                Some(new_grouping_id)
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_colloscope_group_list<V: OrdId>(
    colloscope_group_list: ColloscopeGroupList<V>,
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<ColloscopeGroupList<BackupStudentId>> {
    Some(ColloscopeGroupList {
        name: colloscope_group_list.name,
        groups: colloscope_group_list.groups,
        students_mapping: colloscope_group_list
            .students_mapping
            .into_iter()
            .map(|(student_id, x)| {
                let new_student_id = student_ids.get(&student_id).cloned()?;

                Some((new_student_id, x))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_colloscope_time_slot<T: OrdId>(
    colloscope_time_slot: ColloscopeTimeSlot<T>,
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
) -> Option<ColloscopeTimeSlot<BackupTeacherId>> {
    Some(ColloscopeTimeSlot {
        teacher_id: teacher_ids.get(&colloscope_time_slot.teacher_id).cloned()?,
        start: colloscope_time_slot.start,
        room: colloscope_time_slot.room,
        group_assignments: colloscope_time_slot.group_assignments,
    })
}

fn translate_colloscope_subject<T: OrdId, V: OrdId>(
    colloscope_subject: ColloscopeSubject<T, V>,
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<ColloscopeSubject<BackupTeacherId, BackupStudentId>> {
    Some(ColloscopeSubject {
        time_slots: colloscope_subject
            .time_slots
            .into_iter()
            .map(|x| translate_colloscope_time_slot(x, teacher_ids))
            .collect::<Option<_>>()?,
        group_list: translate_colloscope_group_list(colloscope_subject.group_list, student_ids)?,
    })
}

fn translate_colloscope<T: OrdId, U: OrdId, V: OrdId>(
    colloscope: Colloscope<T, U, V>,
    teacher_ids: &BTreeMap<T, BackupTeacherId>,
    subject_ids: &BTreeMap<U, BackupSubjectId>,
    student_ids: &BTreeMap<V, BackupStudentId>,
) -> Option<Colloscope<BackupTeacherId, BackupSubjectId, BackupStudentId>> {
    Some(Colloscope {
        name: colloscope.name,
        subjects: colloscope
            .subjects
            .into_iter()
            .map(|(subject_id, colloscope_subject)| {
                let new_subject_id = subject_ids.get(&subject_id).cloned()?;

                let new_colloscope_subject =
                    translate_colloscope_subject(colloscope_subject, teacher_ids, student_ids)?;

                Some((new_subject_id, new_colloscope_subject))
            })
            .collect::<Option<_>>()?,
    })
}

fn translate_slot_selection<T: OrdId, U: OrdId>(
    slot_selection: SlotSelection<T, U>,
    subject_ids: &BTreeMap<T, BackupSubjectId>,
    time_slot_ids: &BTreeMap<U, BackupTimeSlotId>,
) -> Option<SlotSelection<BackupSubjectId, BackupTimeSlotId>> {
    Some(SlotSelection {
        subject_id: subject_ids.get(&slot_selection.subject_id).cloned()?,
        slot_groups: slot_selection
            .slot_groups
            .into_iter()
            .map(|slot_group| {
                Some(SlotGroup {
                    slots: slot_group
                        .slots
                        .into_iter()
                        .map(|slot_id| time_slot_ids.get(&slot_id).cloned())
                        .collect::<Option<_>>()?,
                    count: slot_group.count,
                })
            })
            .collect::<Option<_>>()?,
    })
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupWeekPatternId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupTeacherId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupStudentId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupSubjectGroupId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupIncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupGroupListId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupSubjectId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupTimeSlotId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupGroupingId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupGroupingIncompatId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupColloscopeId(u64);
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupSlotSelectionId(u64);

macro_rules! impl_backup_id_from {
    ($id:ty) => {
        impl From<u64> for $id {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}
impl_backup_id_from!(BackupWeekPatternId);
impl_backup_id_from!(BackupTeacherId);
impl_backup_id_from!(BackupStudentId);
impl_backup_id_from!(BackupSubjectGroupId);
impl_backup_id_from!(BackupIncompatId);
impl_backup_id_from!(BackupGroupListId);
impl_backup_id_from!(BackupSubjectId);
impl_backup_id_from!(BackupTimeSlotId);
impl_backup_id_from!(BackupGroupingId);
impl_backup_id_from!(BackupGroupingIncompatId);
impl_backup_id_from!(BackupColloscopeId);
impl_backup_id_from!(BackupSlotSelectionId);

pub struct BackupAssignments {
    pub subject_groups: BTreeMap<BackupSubjectGroupId, BackupSubjectId>,
    pub incompats: BTreeSet<BackupIncompatId>,
}

pub struct BackupData {
    pub next_id: u64,
    pub general_data: GeneralData,
    pub week_patterns: BTreeMap<BackupWeekPatternId, WeekPattern>,
    pub teachers: BTreeMap<BackupTeacherId, Teacher>,
    pub students: BTreeMap<BackupStudentId, Student>,
    pub subject_groups: BTreeMap<BackupSubjectGroupId, SubjectGroup>,
    pub incompats: BTreeMap<BackupIncompatId, Incompat<BackupWeekPatternId>>,
    pub group_lists: BTreeMap<BackupGroupListId, GroupList<BackupStudentId>>,
    pub subjects: BTreeMap<
        BackupSubjectId,
        Subject<BackupSubjectGroupId, BackupIncompatId, BackupGroupListId>,
    >,
    pub time_slots:
        BTreeMap<BackupTimeSlotId, TimeSlot<BackupSubjectId, BackupTeacherId, BackupWeekPatternId>>,
    pub groupings: BTreeMap<BackupGroupingId, Grouping<BackupTimeSlotId>>,
    pub grouping_incompats: BTreeMap<BackupGroupingIncompatId, GroupingIncompat<BackupGroupingId>>,
    pub colloscopes:
        BTreeMap<BackupColloscopeId, Colloscope<BackupTeacherId, BackupSubjectId, BackupStudentId>>,
    pub slot_selections:
        BTreeMap<BackupSlotSelectionId, SlotSelection<BackupSubjectId, BackupTimeSlotId>>,
    pub student_assignments: BTreeMap<BackupStudentId, BackupAssignments>,
}
