use std::collections::VecDeque;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedOperation {
    GeneralData(backend::GeneralData),
    WeekPatterns(AnnotatedWeekPatternsOperation),
    Teachers(AnnotatedTeachersOperation),
    Students(AnnotatedStudentsOperation),
    SubjectGroups(AnnotatedSubjectGroupsOperation),
    Incompats(AnnotatedIncompatsOperation),
    GroupLists(AnnotatedGroupListsOperation),
    Subjects(AnnotatedSubjectsOperation),
    TimeSlots(AnnotatedTimeSlotsOperation),
    Groupings(AnnotatedGroupingsOperation),
    GroupingIncompats(AnnotatedGroupingIncompatsOperation),
    RegisterStudent(AnnotatedRegisterStudentOperation),
    Colloscopes(AnnotatedColloscopesOperation),
    SlotSelections(AnnotatedSlotSelectionsOperation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedWeekPatternsOperation {
    Create(handles::WeekPatternHandle, backend::WeekPattern),
    Remove(handles::WeekPatternHandle),
    Update(handles::WeekPatternHandle, backend::WeekPattern),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedTeachersOperation {
    Create(handles::TeacherHandle, backend::Teacher),
    Remove(handles::TeacherHandle),
    Update(handles::TeacherHandle, backend::Teacher),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedStudentsOperation {
    Create(handles::StudentHandle, backend::Student),
    Remove(handles::StudentHandle),
    Update(handles::StudentHandle, backend::Student),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedSubjectGroupsOperation {
    Create(handles::SubjectGroupHandle, backend::SubjectGroup),
    Remove(handles::SubjectGroupHandle),
    Update(handles::SubjectGroupHandle, backend::SubjectGroup),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedIncompatsOperation {
    Create(
        handles::IncompatHandle,
        backend::Incompat<WeekPatternHandle>,
    ),
    Remove(handles::IncompatHandle),
    Update(
        handles::IncompatHandle,
        backend::Incompat<WeekPatternHandle>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedGroupListsOperation {
    Create(handles::GroupListHandle, backend::GroupList<StudentHandle>),
    Remove(handles::GroupListHandle),
    Update(handles::GroupListHandle, backend::GroupList<StudentHandle>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedSubjectsOperation {
    Create(
        handles::SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    ),
    Remove(handles::SubjectHandle),
    Update(
        handles::SubjectHandle,
        backend::Subject<SubjectGroupHandle, IncompatHandle, GroupListHandle>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedTimeSlotsOperation {
    Create(
        handles::TimeSlotHandle,
        backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
    ),
    Remove(handles::TimeSlotHandle),
    Update(
        handles::TimeSlotHandle,
        backend::TimeSlot<SubjectHandle, TeacherHandle, WeekPatternHandle>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedGroupingsOperation {
    Create(handles::GroupingHandle, backend::Grouping<TimeSlotHandle>),
    Remove(handles::GroupingHandle),
    Update(handles::GroupingHandle, backend::Grouping<TimeSlotHandle>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedGroupingIncompatsOperation {
    Create(
        handles::GroupingIncompatHandle,
        backend::GroupingIncompat<GroupingHandle>,
    ),
    Remove(handles::GroupingIncompatHandle),
    Update(
        handles::GroupingIncompatHandle,
        backend::GroupingIncompat<GroupingHandle>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedRegisterStudentOperation {
    InSubjectGroup(
        handles::StudentHandle,
        handles::SubjectGroupHandle,
        Option<handles::SubjectHandle>,
    ),
    InIncompat(handles::StudentHandle, handles::IncompatHandle, bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedColloscopesOperation {
    Create(
        handles::ColloscopeHandle,
        backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    ),
    Remove(handles::ColloscopeHandle),
    Update(
        handles::ColloscopeHandle,
        backend::Colloscope<TeacherHandle, SubjectHandle, StudentHandle>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedSlotSelectionsOperation {
    Create(
        handles::SlotSelectionHandle,
        backend::SlotSelection<SubjectHandle, TimeSlotHandle>,
    ),
    Remove(handles::SlotSelectionHandle),
    Update(
        handles::SlotSelectionHandle,
        backend::SlotSelection<SubjectHandle, TimeSlotHandle>,
    ),
}

impl AnnotatedWeekPatternsOperation {
    fn annotate<T: backend::Storage>(
        op: WeekPatternsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            WeekPatternsOperation::Create(pattern) => {
                let handle = handle_managers.week_patterns.create_handle();
                AnnotatedWeekPatternsOperation::Create(handle, pattern)
            }
            WeekPatternsOperation::Remove(handle) => AnnotatedWeekPatternsOperation::Remove(handle),
            WeekPatternsOperation::Update(handle, pattern) => {
                AnnotatedWeekPatternsOperation::Update(handle, pattern)
            }
        }
    }
}

impl AnnotatedTeachersOperation {
    fn annotate<T: backend::Storage>(
        op: TeachersOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            TeachersOperation::Create(teacher) => {
                let handle = handle_managers.teachers.create_handle();
                AnnotatedTeachersOperation::Create(handle, teacher)
            }
            TeachersOperation::Remove(handle) => AnnotatedTeachersOperation::Remove(handle),
            TeachersOperation::Update(handle, teacher) => {
                AnnotatedTeachersOperation::Update(handle, teacher)
            }
        }
    }
}

impl AnnotatedStudentsOperation {
    fn annotate<T: backend::Storage>(
        op: StudentsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            StudentsOperation::Create(student) => {
                let handle = handle_managers.students.create_handle();
                AnnotatedStudentsOperation::Create(handle, student)
            }
            StudentsOperation::Remove(handle) => AnnotatedStudentsOperation::Remove(handle),
            StudentsOperation::Update(handle, student) => {
                AnnotatedStudentsOperation::Update(handle, student)
            }
        }
    }
}

impl AnnotatedSubjectGroupsOperation {
    fn annotate<T: backend::Storage>(
        op: SubjectGroupsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            SubjectGroupsOperation::Create(subject_group) => {
                let handle = handle_managers.subject_groups.create_handle();
                AnnotatedSubjectGroupsOperation::Create(handle, subject_group)
            }
            SubjectGroupsOperation::Remove(handle) => {
                AnnotatedSubjectGroupsOperation::Remove(handle)
            }
            SubjectGroupsOperation::Update(handle, subject_group) => {
                AnnotatedSubjectGroupsOperation::Update(handle, subject_group)
            }
        }
    }
}

impl AnnotatedIncompatsOperation {
    fn annotate<T: backend::Storage>(
        op: IncompatsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            IncompatsOperation::Create(incompat) => {
                let handle = handle_managers.incompats.create_handle();
                AnnotatedIncompatsOperation::Create(handle, incompat)
            }
            IncompatsOperation::Remove(handle) => AnnotatedIncompatsOperation::Remove(handle),
            IncompatsOperation::Update(handle, incompat) => {
                AnnotatedIncompatsOperation::Update(handle, incompat)
            }
        }
    }
}

impl AnnotatedGroupListsOperation {
    fn annotate<T: backend::Storage>(
        op: GroupListsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            GroupListsOperation::Create(group_list) => {
                let handle = handle_managers.group_lists.create_handle();
                AnnotatedGroupListsOperation::Create(handle, group_list)
            }
            GroupListsOperation::Remove(handle) => AnnotatedGroupListsOperation::Remove(handle),
            GroupListsOperation::Update(handle, group_list) => {
                AnnotatedGroupListsOperation::Update(handle, group_list)
            }
        }
    }
}

impl AnnotatedSubjectsOperation {
    fn annotate<T: backend::Storage>(
        op: SubjectsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            SubjectsOperation::Create(subject) => {
                let handle = handle_managers.subjects.create_handle();
                AnnotatedSubjectsOperation::Create(handle, subject)
            }
            SubjectsOperation::Remove(handle) => AnnotatedSubjectsOperation::Remove(handle),
            SubjectsOperation::Update(handle, subject) => {
                AnnotatedSubjectsOperation::Update(handle, subject)
            }
        }
    }
}

impl AnnotatedTimeSlotsOperation {
    fn annotate<T: backend::Storage>(
        op: TimeSlotsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            TimeSlotsOperation::Create(time_slot) => {
                let handle = handle_managers.time_slots.create_handle();
                AnnotatedTimeSlotsOperation::Create(handle, time_slot)
            }
            TimeSlotsOperation::Remove(handle) => AnnotatedTimeSlotsOperation::Remove(handle),
            TimeSlotsOperation::Update(handle, time_slot) => {
                AnnotatedTimeSlotsOperation::Update(handle, time_slot)
            }
        }
    }
}

impl AnnotatedGroupingsOperation {
    fn annotate<T: backend::Storage>(
        op: GroupingsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            GroupingsOperation::Create(grouping) => {
                let handle = handle_managers.groupings.create_handle();
                AnnotatedGroupingsOperation::Create(handle, grouping)
            }
            GroupingsOperation::Remove(handle) => AnnotatedGroupingsOperation::Remove(handle),
            GroupingsOperation::Update(handle, grouping) => {
                AnnotatedGroupingsOperation::Update(handle, grouping)
            }
        }
    }
}

impl AnnotatedGroupingIncompatsOperation {
    fn annotate<T: backend::Storage>(
        op: GroupingIncompatsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            GroupingIncompatsOperation::Create(grouping_incompat) => {
                let handle = handle_managers.grouping_incompats.create_handle();
                AnnotatedGroupingIncompatsOperation::Create(handle, grouping_incompat)
            }
            GroupingIncompatsOperation::Remove(handle) => {
                AnnotatedGroupingIncompatsOperation::Remove(handle)
            }
            GroupingIncompatsOperation::Update(handle, grouping_incompat) => {
                AnnotatedGroupingIncompatsOperation::Update(handle, grouping_incompat)
            }
        }
    }
}

impl AnnotatedRegisterStudentOperation {
    fn annotate<T: backend::Storage>(
        op: RegisterStudentOperation,
        _handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            RegisterStudentOperation::InSubjectGroup(
                student_handle,
                subject_group_handle,
                subject_handle,
            ) => AnnotatedRegisterStudentOperation::InSubjectGroup(
                student_handle,
                subject_group_handle,
                subject_handle,
            ),
            RegisterStudentOperation::InIncompat(student_handle, incompat_handle, enabled) => {
                AnnotatedRegisterStudentOperation::InIncompat(
                    student_handle,
                    incompat_handle,
                    enabled,
                )
            }
        }
    }
}

impl AnnotatedColloscopesOperation {
    fn annotate<T: backend::Storage>(
        op: ColloscopesOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            ColloscopesOperation::Create(colloscope) => {
                let handle = handle_managers.colloscopes.create_handle();
                AnnotatedColloscopesOperation::Create(handle, colloscope)
            }
            ColloscopesOperation::Remove(handle) => AnnotatedColloscopesOperation::Remove(handle),
            ColloscopesOperation::Update(handle, colloscope) => {
                AnnotatedColloscopesOperation::Update(handle, colloscope)
            }
        }
    }
}

impl AnnotatedSlotSelectionsOperation {
    fn annotate<T: backend::Storage>(
        op: SlotSelectionsOperation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            SlotSelectionsOperation::Create(slot_selection) => {
                let handle = handle_managers.slot_selections.create_handle();
                AnnotatedSlotSelectionsOperation::Create(handle, slot_selection)
            }
            SlotSelectionsOperation::Remove(handle) => {
                AnnotatedSlotSelectionsOperation::Remove(handle)
            }
            SlotSelectionsOperation::Update(handle, slot_selection) => {
                AnnotatedSlotSelectionsOperation::Update(handle, slot_selection)
            }
        }
    }
}

impl AnnotatedOperation {
    pub fn annotate<T: backend::Storage>(
        op: Operation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            Operation::GeneralData(data) => AnnotatedOperation::GeneralData(data),
            Operation::WeekPatterns(op) => AnnotatedOperation::WeekPatterns(
                AnnotatedWeekPatternsOperation::annotate(op, handle_managers),
            ),
            Operation::Teachers(op) => AnnotatedOperation::Teachers(
                AnnotatedTeachersOperation::annotate(op, handle_managers),
            ),
            Operation::Students(op) => AnnotatedOperation::Students(
                AnnotatedStudentsOperation::annotate(op, handle_managers),
            ),
            Operation::SubjectGroups(op) => AnnotatedOperation::SubjectGroups(
                AnnotatedSubjectGroupsOperation::annotate(op, handle_managers),
            ),
            Operation::Incompats(op) => AnnotatedOperation::Incompats(
                AnnotatedIncompatsOperation::annotate(op, handle_managers),
            ),
            Operation::GroupLists(op) => AnnotatedOperation::GroupLists(
                AnnotatedGroupListsOperation::annotate(op, handle_managers),
            ),
            Operation::Subjects(op) => AnnotatedOperation::Subjects(
                AnnotatedSubjectsOperation::annotate(op, handle_managers),
            ),
            Operation::TimeSlots(op) => AnnotatedOperation::TimeSlots(
                AnnotatedTimeSlotsOperation::annotate(op, handle_managers),
            ),
            Operation::Groupings(op) => AnnotatedOperation::Groupings(
                AnnotatedGroupingsOperation::annotate(op, handle_managers),
            ),
            Operation::GroupingIncompats(op) => AnnotatedOperation::GroupingIncompats(
                AnnotatedGroupingIncompatsOperation::annotate(op, handle_managers),
            ),
            Operation::RegisterStudent(op) => AnnotatedOperation::RegisterStudent(
                AnnotatedRegisterStudentOperation::annotate(op, handle_managers),
            ),
            Operation::Colloscopes(op) => AnnotatedOperation::Colloscopes(
                AnnotatedColloscopesOperation::annotate(op, handle_managers),
            ),
            Operation::SlotSelections(op) => AnnotatedOperation::SlotSelections(
                AnnotatedSlotSelectionsOperation::annotate(op, handle_managers),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReversibleOperation {
    pub forward: AnnotatedOperation,
    pub backward: AnnotatedOperation,
}

impl ReversibleOperation {
    pub fn rev(&self) -> Self {
        ReversibleOperation {
            forward: self.backward.clone(),
            backward: self.forward.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatedOperations(Vec<ReversibleOperation>);

impl AggregatedOperations {
    pub fn new(ops: Vec<ReversibleOperation>) -> Self {
        AggregatedOperations(ops)
    }

    pub fn rev(&self) -> Self {
        AggregatedOperations(self.0.iter().rev().map(|x| x.rev()).collect())
    }

    pub fn inner(&self) -> &Vec<ReversibleOperation> {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ModificationHistory {
    history: VecDeque<AggregatedOperations>,
    history_pointer: usize,
    max_history_size: Option<usize>,
}

impl ModificationHistory {
    fn truncate_history_as_needed(&mut self) {
        if let Some(max_hist_size) = self.max_history_size {
            if max_hist_size >= self.history.len() {
                return;
            }

            // Try to keep undo history as a priority (rather than redo history)
            // So we remove the beginning of the queue only if we really can't keep it
            if self.history_pointer > max_hist_size {
                let split_point = self.history_pointer - max_hist_size;
                let new_history = self.history.split_off(split_point);
                self.history = new_history;

                self.history_pointer = max_hist_size;
            }

            self.history.truncate(max_hist_size);
        }
    }
}

impl ModificationHistory {
    pub fn new() -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size: None,
        }
    }

    pub fn with_max_history_size(max_history_size: Option<usize>) -> Self {
        ModificationHistory {
            history: std::collections::VecDeque::new(),
            history_pointer: 0,
            max_history_size,
        }
    }

    pub fn get_max_history_size(&self) -> Option<usize> {
        self.max_history_size
    }

    pub fn set_max_history_size(&mut self, max_history_size: Option<usize>) {
        self.max_history_size = max_history_size;

        self.truncate_history_as_needed();
    }

    pub fn apply(&mut self, aggregated_ops: AggregatedOperations) {
        self.history.truncate(self.history_pointer);

        self.history_pointer += 1;
        self.history.push_back(aggregated_ops);

        self.truncate_history_as_needed();
    }

    pub fn can_undo(&self) -> bool {
        self.history_pointer > 0
    }

    pub fn can_redo(&self) -> bool {
        self.history_pointer < self.history.len()
    }

    pub fn undo(&mut self) -> Option<AggregatedOperations> {
        if !self.can_undo() {
            return None;
        }

        self.history_pointer -= 1;

        assert!(self.history_pointer < self.history.len());

        let last_ops = self.history[self.history_pointer].clone();

        Some(last_ops.rev())
    }

    pub fn redo(&mut self) -> Option<AggregatedOperations> {
        if !self.can_redo() {
            return None;
        }

        let new_ops = self.history[self.history_pointer].clone();
        self.history_pointer += 1;

        Some(new_ops)
    }

    pub fn build_aggregated_ops(&self) -> AggregatedOperations {
        AggregatedOperations::new(
            self.history
                .iter()
                .take(self.history_pointer)
                .flat_map(|aggregated_ops| aggregated_ops.inner().iter())
                .cloned()
                .collect(),
        )
    }

    pub fn clear_past_history(&mut self) {
        let new_history = self.history.split_off(self.history_pointer);
        self.history = new_history;
        self.history_pointer = 0;
    }
}
