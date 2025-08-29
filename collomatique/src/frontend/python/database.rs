use pyo3::exceptions::{PyException, PyValueError};
use std::collections::BTreeMap;

use super::*;

mod classes;
use classes::*;

mod utils;

#[pymodule]
pub fn collomatique(m: &Bound<'_, PyModule>) -> PyResult<()> {
    use utils::*;

    m.add_class::<GeneralData>()?;
    m.add_class::<WeekPattern>()?;
    m.add_class::<Teacher>()?;
    m.add_class::<Student>()?;
    m.add_class::<SubjectGroup>()?;
    m.add_class::<Weekday>()?;
    m.add_class::<Time>()?;
    m.add_class::<SlotStart>()?;
    m.add_class::<IncompatSlot>()?;
    m.add_class::<Incompat>()?;
    m.add_class::<Group>()?;
    m.add_class::<GroupList>()?;
    m.add_class::<Subject>()?;
    m.add_class::<TimeSlot>()?;
    m.add_class::<Grouping>()?;
    m.add_class::<GroupingIncompat>()?;
    m.add_class::<SlotGroup>()?;
    m.add_class::<SlotSelection>()?;
    m.add_class::<BalancingConstraints>()?;
    m.add_class::<BalancingSlotSelections>()?;

    m.add_function(wrap_pyfunction!(extract_name_parts, m)?)?;
    m.add_function(wrap_pyfunction!(load_csv, m)?)?;

    Ok(())
}

#[pyclass]
pub struct Database {
    sender: Sender<Job>,
}

#[pymethods]
impl Database {
    fn undo(self_: PyRef<'_, Self>) -> PyResult<()> {
        let Answer::Undo =
            SessionConnection::send_command(self_.py(), &self_.sender, Command::Undo)?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn redo(self_: PyRef<'_, Self>) -> PyResult<()> {
        let Answer::Redo =
            SessionConnection::send_command(self_.py(), &self_.sender, Command::Redo)?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn general_data_get(self_: PyRef<'_, Self>) -> PyResult<GeneralData> {
        let Answer::GeneralData(GeneralDataAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GeneralData(GeneralDataCommand::Get),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn general_data_set(self_: PyRef<'_, Self>, general_data: GeneralData) -> PyResult<()> {
        let Answer::GeneralData(GeneralDataAnswer::Set) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GeneralData(GeneralDataCommand::Set(general_data)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn week_patterns_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<WeekPatternHandle, WeekPattern>> {
        let Answer::WeekPatterns(WeekPatternsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::WeekPatterns(WeekPatternsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn week_patterns_get(
        self_: PyRef<'_, Self>,
        handle: WeekPatternHandle,
    ) -> PyResult<WeekPattern> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn week_patterns_create(
        self_: PyRef<'_, Self>,
        pattern: WeekPattern,
    ) -> PyResult<WeekPatternHandle> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::WeekPatterns(WeekPatternsCommand::Create(pattern)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn week_patterns_update(
        self_: PyRef<'_, Self>,
        handle: WeekPatternHandle,
        pattern: WeekPattern,
    ) -> PyResult<()> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Update(handle, pattern)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn week_patterns_remove(self_: PyRef<'_, Self>, handle: WeekPatternHandle) -> PyResult<()> {
        let Answer::WeekPatterns(WeekPatternsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::WeekPatterns(WeekPatternsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn teachers_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<TeacherHandle, Teacher>> {
        let Answer::Teachers(TeachersAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn teachers_get(self_: PyRef<'_, Self>, handle: TeacherHandle) -> PyResult<Teacher> {
        let Answer::Teachers(TeachersAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn teachers_create(self_: PyRef<'_, Self>, teacher: Teacher) -> PyResult<TeacherHandle> {
        let Answer::Teachers(TeachersAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Create(teacher)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn teachers_update(
        self_: PyRef<'_, Self>,
        handle: TeacherHandle,
        teacher: Teacher,
    ) -> PyResult<()> {
        let Answer::Teachers(TeachersAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Update(handle, teacher)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn teachers_remove(self_: PyRef<'_, Self>, handle: TeacherHandle) -> PyResult<()> {
        let Answer::Teachers(TeachersAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Teachers(TeachersCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn students_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<StudentHandle, Student>> {
        let Answer::Students(StudentsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn students_get(self_: PyRef<'_, Self>, handle: StudentHandle) -> PyResult<Student> {
        let Answer::Students(StudentsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn students_create(self_: PyRef<'_, Self>, student: Student) -> PyResult<StudentHandle> {
        let Answer::Students(StudentsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Create(student)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn students_update(
        self_: PyRef<'_, Self>,
        handle: StudentHandle,
        student: Student,
    ) -> PyResult<()> {
        let Answer::Students(StudentsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Update(handle, student)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn students_remove(self_: PyRef<'_, Self>, handle: StudentHandle) -> PyResult<()> {
        let Answer::Students(StudentsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Students(StudentsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subject_groups_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<SubjectGroupHandle, SubjectGroup>> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SubjectGroups(SubjectGroupsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subject_groups_get(
        self_: PyRef<'_, Self>,
        handle: SubjectGroupHandle,
    ) -> PyResult<SubjectGroup> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subject_groups_create(
        self_: PyRef<'_, Self>,
        subject_group: SubjectGroup,
    ) -> PyResult<SubjectGroupHandle> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SubjectGroups(SubjectGroupsCommand::Create(subject_group)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn subject_groups_update(
        self_: PyRef<'_, Self>,
        handle: SubjectGroupHandle,
        subject_group: SubjectGroup,
    ) -> PyResult<()> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Update(handle, subject_group)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subject_groups_remove(self_: PyRef<'_, Self>, handle: SubjectGroupHandle) -> PyResult<()> {
        let Answer::SubjectGroups(SubjectGroupsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SubjectGroups(SubjectGroupsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn incompats_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<IncompatHandle, Incompat>> {
        let Answer::Incompats(IncompatsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Incompats(IncompatsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn incompats_get(self_: PyRef<'_, Self>, handle: IncompatHandle) -> PyResult<Incompat> {
        let Answer::Incompats(IncompatsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Incompats(IncompatsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn incompats_create(self_: PyRef<'_, Self>, incompat: Incompat) -> PyResult<IncompatHandle> {
        let Answer::Incompats(IncompatsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Incompats(IncompatsCommand::Create(incompat)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn incompats_update(
        self_: PyRef<'_, Self>,
        handle: IncompatHandle,
        incompat: Incompat,
    ) -> PyResult<()> {
        let Answer::Incompats(IncompatsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Incompats(IncompatsCommand::Update(handle, incompat)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn incompats_remove(self_: PyRef<'_, Self>, handle: IncompatHandle) -> PyResult<()> {
        let Answer::Incompats(IncompatsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Incompats(IncompatsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn group_lists_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<GroupListHandle, GroupList>> {
        let Answer::GroupLists(GroupListsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GroupLists(GroupListsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn group_lists_get(self_: PyRef<'_, Self>, handle: GroupListHandle) -> PyResult<GroupList> {
        let Answer::GroupLists(GroupListsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GroupLists(GroupListsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn group_lists_create(
        self_: PyRef<'_, Self>,
        group_list: GroupList,
    ) -> PyResult<GroupListHandle> {
        let Answer::GroupLists(GroupListsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GroupLists(GroupListsCommand::Create(group_list)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn group_lists_update(
        self_: PyRef<'_, Self>,
        handle: GroupListHandle,
        group_list: GroupList,
    ) -> PyResult<()> {
        let Answer::GroupLists(GroupListsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GroupLists(GroupListsCommand::Update(handle, group_list)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn group_lists_remove(self_: PyRef<'_, Self>, handle: GroupListHandle) -> PyResult<()> {
        let Answer::GroupLists(GroupListsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::GroupLists(GroupListsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subjects_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<SubjectHandle, Subject>> {
        let Answer::Subjects(SubjectsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Subjects(SubjectsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subjects_get(self_: PyRef<'_, Self>, handle: SubjectHandle) -> PyResult<Subject> {
        let Answer::Subjects(SubjectsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Subjects(SubjectsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn subjects_create(self_: PyRef<'_, Self>, subject: Subject) -> PyResult<SubjectHandle> {
        let Answer::Subjects(SubjectsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Subjects(SubjectsCommand::Create(subject)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn subjects_update(
        self_: PyRef<'_, Self>,
        handle: SubjectHandle,
        subject: Subject,
    ) -> PyResult<()> {
        let Answer::Subjects(SubjectsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Subjects(SubjectsCommand::Update(handle, subject)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subjects_remove(self_: PyRef<'_, Self>, handle: SubjectHandle) -> PyResult<()> {
        let Answer::Subjects(SubjectsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Subjects(SubjectsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn time_slots_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<TimeSlotHandle, TimeSlot>> {
        let Answer::TimeSlots(TimeSlotsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::TimeSlots(TimeSlotsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn time_slots_get(self_: PyRef<'_, Self>, handle: TimeSlotHandle) -> PyResult<TimeSlot> {
        let Answer::TimeSlots(TimeSlotsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::TimeSlots(TimeSlotsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn time_slots_create(self_: PyRef<'_, Self>, time_slot: TimeSlot) -> PyResult<TimeSlotHandle> {
        let Answer::TimeSlots(TimeSlotsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::TimeSlots(TimeSlotsCommand::Create(time_slot)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn time_slots_update(
        self_: PyRef<'_, Self>,
        handle: TimeSlotHandle,
        time_slot: TimeSlot,
    ) -> PyResult<()> {
        let Answer::TimeSlots(TimeSlotsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::TimeSlots(TimeSlotsCommand::Update(handle, time_slot)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn time_slots_remove(self_: PyRef<'_, Self>, handle: TimeSlotHandle) -> PyResult<()> {
        let Answer::TimeSlots(TimeSlotsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::TimeSlots(TimeSlotsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn groupings_get_all(self_: PyRef<'_, Self>) -> PyResult<BTreeMap<GroupingHandle, Grouping>> {
        let Answer::Groupings(GroupingsAnswer::GetAll(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Groupings(GroupingsCommand::GetAll),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn groupings_get(self_: PyRef<'_, Self>, handle: GroupingHandle) -> PyResult<Grouping> {
        let Answer::Groupings(GroupingsAnswer::Get(val)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Groupings(GroupingsCommand::Get(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn groupings_create(self_: PyRef<'_, Self>, grouping: Grouping) -> PyResult<GroupingHandle> {
        let Answer::Groupings(GroupingsAnswer::Create(handle)) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Groupings(GroupingsCommand::Create(grouping)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn groupings_update(
        self_: PyRef<'_, Self>,
        handle: GroupingHandle,
        grouping: Grouping,
    ) -> PyResult<()> {
        let Answer::Groupings(GroupingsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Groupings(GroupingsCommand::Update(handle, grouping)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn groupings_remove(self_: PyRef<'_, Self>, handle: GroupingHandle) -> PyResult<()> {
        let Answer::Groupings(GroupingsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::Groupings(GroupingsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn grouping_incompats_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<GroupingIncompatHandle, GroupingIncompat>> {
        let Answer::GroupingIncompats(GroupingIncompatsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::GroupingIncompats(GroupingIncompatsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn grouping_incompats_get(
        self_: PyRef<'_, Self>,
        handle: GroupingIncompatHandle,
    ) -> PyResult<GroupingIncompat> {
        let Answer::GroupingIncompats(GroupingIncompatsAnswer::Get(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::GroupingIncompats(GroupingIncompatsCommand::Get(handle)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn grouping_incompats_create(
        self_: PyRef<'_, Self>,
        grouping_incompat: GroupingIncompat,
    ) -> PyResult<GroupingIncompatHandle> {
        let Answer::GroupingIncompats(GroupingIncompatsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::GroupingIncompats(GroupingIncompatsCommand::Create(grouping_incompat)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn grouping_incompats_update(
        self_: PyRef<'_, Self>,
        handle: GroupingIncompatHandle,
        grouping_incompat: GroupingIncompat,
    ) -> PyResult<()> {
        let Answer::GroupingIncompats(GroupingIncompatsAnswer::Update) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::GroupingIncompats(GroupingIncompatsCommand::Update(
                    handle,
                    grouping_incompat,
                )),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn grouping_incompats_remove(
        self_: PyRef<'_, Self>,
        handle: GroupingIncompatHandle,
    ) -> PyResult<()> {
        let Answer::GroupingIncompats(GroupingIncompatsAnswer::Remove) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::GroupingIncompats(GroupingIncompatsCommand::Remove(handle)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn subject_group_for_student_get(
        self_: PyRef<'_, Self>,
        student_handle: StudentHandle,
        subject_group_handle: SubjectGroupHandle,
    ) -> PyResult<Option<SubjectHandle>> {
        let Answer::RegisterStudent(RegisterStudentAnswer::InSubjectGroupGet(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::RegisterStudent(RegisterStudentCommand::InSubjectGroupGet(
                    student_handle,
                    subject_group_handle,
                )),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    #[pyo3(signature=(student_handle, subject_group_handle, subject_handle))]
    fn subject_group_for_student_set(
        self_: PyRef<'_, Self>,
        student_handle: StudentHandle,
        subject_group_handle: SubjectGroupHandle,
        subject_handle: Option<SubjectHandle>,
    ) -> PyResult<()> {
        let Answer::RegisterStudent(RegisterStudentAnswer::InSubjectGroupSet) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::RegisterStudent(RegisterStudentCommand::InSubjectGroupSet(
                    student_handle,
                    subject_group_handle,
                    subject_handle,
                )),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn incompat_for_student_get(
        self_: PyRef<'_, Self>,
        student_handle: StudentHandle,
        incompat_handle: IncompatHandle,
    ) -> PyResult<bool> {
        let Answer::RegisterStudent(RegisterStudentAnswer::InIncompatGet(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::RegisterStudent(RegisterStudentCommand::InIncompatGet(
                    student_handle,
                    incompat_handle,
                )),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn incompat_for_student_set(
        self_: PyRef<'_, Self>,
        student_handle: StudentHandle,
        incompat_handle: IncompatHandle,
        enabled: bool,
    ) -> PyResult<()> {
        let Answer::RegisterStudent(RegisterStudentAnswer::InIncompatSet) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::RegisterStudent(RegisterStudentCommand::InIncompatSet(
                    student_handle,
                    incompat_handle,
                    enabled,
                )),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn slot_selections_get_all(
        self_: PyRef<'_, Self>,
    ) -> PyResult<BTreeMap<SlotSelectionHandle, SlotSelection>> {
        let Answer::SlotSelections(SlotSelectionsAnswer::GetAll(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SlotSelections(SlotSelectionsCommand::GetAll),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn slot_selections_get(
        self_: PyRef<'_, Self>,
        handle: SlotSelectionHandle,
    ) -> PyResult<SlotSelection> {
        let Answer::SlotSelections(SlotSelectionsAnswer::Get(val)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SlotSelections(SlotSelectionsCommand::Get(handle)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(val)
    }

    fn slot_selections_create(
        self_: PyRef<'_, Self>,
        slot_selection: SlotSelection,
    ) -> PyResult<SlotSelectionHandle> {
        let Answer::SlotSelections(SlotSelectionsAnswer::Create(handle)) =
            SessionConnection::send_command(
                self_.py(),
                &self_.sender,
                Command::SlotSelections(SlotSelectionsCommand::Create(slot_selection)),
            )?
        else {
            panic!("Bad answer type");
        };

        Ok(handle)
    }

    fn slot_selections_update(
        self_: PyRef<'_, Self>,
        handle: SlotSelectionHandle,
        grouping_incompat: SlotSelection,
    ) -> PyResult<()> {
        let Answer::SlotSelections(SlotSelectionsAnswer::Update) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SlotSelections(SlotSelectionsCommand::Update(handle, grouping_incompat)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }

    fn slot_selections_remove(self_: PyRef<'_, Self>, handle: SlotSelectionHandle) -> PyResult<()> {
        let Answer::SlotSelections(SlotSelectionsAnswer::Remove) = SessionConnection::send_command(
            self_.py(),
            &self_.sender,
            Command::SlotSelections(SlotSelectionsCommand::Remove(handle)),
        )?
        else {
            panic!("Bad answer type");
        };

        Ok(())
    }
}

use std::sync::mpsc::{self, Receiver, Sender};

use crate::frontend::state::update::ReturnHandle;
use crate::frontend::state::{self, Operation, RedoError, UndoError, UpdateError};
use crate::json::{self, Id2Error, IdError};

#[derive(Debug, Clone)]
pub enum Command {
    GeneralData(GeneralDataCommand),
    WeekPatterns(WeekPatternsCommand),
    Teachers(TeachersCommand),
    Students(StudentsCommand),
    SubjectGroups(SubjectGroupsCommand),
    Incompats(IncompatsCommand),
    GroupLists(GroupListsCommand),
    Subjects(SubjectsCommand),
    TimeSlots(TimeSlotsCommand),
    Groupings(GroupingsCommand),
    GroupingIncompats(GroupingIncompatsCommand),
    RegisterStudent(RegisterStudentCommand),
    SlotSelections(SlotSelectionsCommand),
    Undo,
    Redo,
    Exit,
}

#[derive(Debug, Clone)]
pub enum GeneralDataCommand {
    Get,
    Set(GeneralData),
}

#[derive(Debug, Clone)]
pub enum WeekPatternsCommand {
    GetAll,
    Get(WeekPatternHandle),
    Create(WeekPattern),
    Update(WeekPatternHandle, WeekPattern),
    Remove(WeekPatternHandle),
}

#[derive(Debug, Clone)]
pub enum TeachersCommand {
    GetAll,
    Get(TeacherHandle),
    Create(Teacher),
    Update(TeacherHandle, Teacher),
    Remove(TeacherHandle),
}

#[derive(Debug, Clone)]
pub enum StudentsCommand {
    GetAll,
    Get(StudentHandle),
    Create(Student),
    Update(StudentHandle, Student),
    Remove(StudentHandle),
}

#[derive(Debug, Clone)]
pub enum SubjectGroupsCommand {
    GetAll,
    Get(SubjectGroupHandle),
    Create(SubjectGroup),
    Update(SubjectGroupHandle, SubjectGroup),
    Remove(SubjectGroupHandle),
}

#[derive(Debug, Clone)]
pub enum IncompatsCommand {
    GetAll,
    Get(IncompatHandle),
    Create(Incompat),
    Update(IncompatHandle, Incompat),
    Remove(IncompatHandle),
}

#[derive(Debug, Clone)]
pub enum GroupListsCommand {
    GetAll,
    Get(GroupListHandle),
    Create(GroupList),
    Update(GroupListHandle, GroupList),
    Remove(GroupListHandle),
}

#[derive(Debug, Clone)]
pub enum SubjectsCommand {
    GetAll,
    Get(SubjectHandle),
    Create(Subject),
    Update(SubjectHandle, Subject),
    Remove(SubjectHandle),
}

#[derive(Debug, Clone)]
pub enum TimeSlotsCommand {
    GetAll,
    Get(TimeSlotHandle),
    Create(TimeSlot),
    Update(TimeSlotHandle, TimeSlot),
    Remove(TimeSlotHandle),
}

#[derive(Debug, Clone)]
pub enum GroupingsCommand {
    GetAll,
    Get(GroupingHandle),
    Create(Grouping),
    Update(GroupingHandle, Grouping),
    Remove(GroupingHandle),
}

#[derive(Debug, Clone)]
pub enum GroupingIncompatsCommand {
    GetAll,
    Get(GroupingIncompatHandle),
    Create(GroupingIncompat),
    Update(GroupingIncompatHandle, GroupingIncompat),
    Remove(GroupingIncompatHandle),
}

#[derive(Debug, Clone)]
pub enum RegisterStudentCommand {
    InSubjectGroupGet(StudentHandle, SubjectGroupHandle),
    InSubjectGroupSet(StudentHandle, SubjectGroupHandle, Option<SubjectHandle>),
    InIncompatGet(StudentHandle, IncompatHandle),
    InIncompatSet(StudentHandle, IncompatHandle, bool),
}

#[derive(Debug, Clone)]
pub enum SlotSelectionsCommand {
    GetAll,
    Get(SlotSelectionHandle),
    Create(SlotSelection),
    Update(SlotSelectionHandle, SlotSelection),
    Remove(SlotSelectionHandle),
}

#[derive(Debug)]
struct PythonError {
    int_err: Box<dyn std::error::Error + Send>,
}

impl std::fmt::Display for PythonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &*self.int_err)
    }
}

impl std::error::Error for PythonError {}

#[derive(Debug)]
pub enum Answer {
    GeneralData(GeneralDataAnswer),
    WeekPatterns(WeekPatternsAnswer),
    Teachers(TeachersAnswer),
    Students(StudentsAnswer),
    SubjectGroups(SubjectGroupsAnswer),
    Incompats(IncompatsAnswer),
    GroupLists(GroupListsAnswer),
    Subjects(SubjectsAnswer),
    TimeSlots(TimeSlotsAnswer),
    Groupings(GroupingsAnswer),
    GroupingIncompats(GroupingIncompatsAnswer),
    RegisterStudent(RegisterStudentAnswer),
    SlotSelections(SlotSelectionsAnswer),
    Undo,
    Redo,
}

#[derive(Debug)]
pub enum GeneralDataAnswer {
    Get(GeneralData),
    Set,
}

#[derive(Debug)]
pub enum WeekPatternsAnswer {
    GetAll(BTreeMap<WeekPatternHandle, WeekPattern>),
    Get(WeekPattern),
    Create(WeekPatternHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum TeachersAnswer {
    GetAll(BTreeMap<TeacherHandle, Teacher>),
    Get(Teacher),
    Create(TeacherHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum StudentsAnswer {
    GetAll(BTreeMap<StudentHandle, Student>),
    Get(Student),
    Create(StudentHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum SubjectGroupsAnswer {
    GetAll(BTreeMap<SubjectGroupHandle, SubjectGroup>),
    Get(SubjectGroup),
    Create(SubjectGroupHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum IncompatsAnswer {
    GetAll(BTreeMap<IncompatHandle, Incompat>),
    Get(Incompat),
    Create(IncompatHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum GroupListsAnswer {
    GetAll(BTreeMap<GroupListHandle, GroupList>),
    Get(GroupList),
    Create(GroupListHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum SubjectsAnswer {
    GetAll(BTreeMap<SubjectHandle, Subject>),
    Get(Subject),
    Create(SubjectHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum TimeSlotsAnswer {
    GetAll(BTreeMap<TimeSlotHandle, TimeSlot>),
    Get(TimeSlot),
    Create(TimeSlotHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum GroupingsAnswer {
    GetAll(BTreeMap<GroupingHandle, Grouping>),
    Get(Grouping),
    Create(GroupingHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum GroupingIncompatsAnswer {
    GetAll(BTreeMap<GroupingIncompatHandle, GroupingIncompat>),
    Get(GroupingIncompat),
    Create(GroupingIncompatHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub enum RegisterStudentAnswer {
    InSubjectGroupGet(Option<SubjectHandle>),
    InSubjectGroupSet,
    InIncompatGet(bool),
    InIncompatSet,
}

#[derive(Debug)]
pub enum SlotSelectionsAnswer {
    GetAll(BTreeMap<SlotSelectionHandle, SlotSelection>),
    Get(SlotSelection),
    Create(SlotSelectionHandle),
    Update,
    Remove,
}

#[derive(Debug)]
pub struct Job {
    command: Command,
    answer: Sender<PyResult<Answer>>,
}

#[derive(Debug)]
pub struct SessionConnection<'scope> {
    queue_sender: Sender<Job>,
    thread: Option<std::thread::ScopedJoinHandle<'scope, ()>>,
}

impl<'scope> Drop for SessionConnection<'scope> {
    fn drop(&mut self) {
        if self.thread.is_some() {
            drop(Self::send_command_internal(
                &self.queue_sender,
                Command::Exit,
            ));
        }
    }
}

impl<'scope> SessionConnection<'scope> {
    pub fn new<T: state::Manager>(
        scope: &'scope std::thread::Scope<'scope, '_>,
        manager: &'scope mut T,
    ) -> SessionConnection<'scope> {
        let (queue_sender, queue_receiver) = mpsc::channel();

        let thread = Some(scope.spawn(move || {
            SessionConnection::thread_func(queue_receiver, manager);
        }));

        SessionConnection {
            queue_sender,
            thread,
        }
    }

    pub fn python_database(&self) -> Database {
        Database {
            sender: self.queue_sender.clone(),
        }
    }

    pub fn join(mut self) {
        drop(Self::send_command_internal(
            &self.queue_sender,
            Command::Exit,
        ));
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }

    fn thread_func<T: state::Manager>(queue_receiver: Receiver<Job>, manager: &'scope mut T) {
        while let Ok(job) = queue_receiver.recv() {
            if let Command::Exit = &job.command {
                return;
            }

            let answer_data = Self::execute_job(&job.command, manager);
            job.answer.send(answer_data).unwrap();
        }
    }

    fn execute_general_data_job<T: state::Manager>(
        general_data_command: &GeneralDataCommand,
        manager: &mut T,
    ) -> PyResult<GeneralDataAnswer> {
        match general_data_command {
            GeneralDataCommand::Get => {
                let general_data = manager
                    .general_data_get()
                    .map_err(|e| PyException::new_err(e.to_string()))?;

                Ok(GeneralDataAnswer::Get(general_data.into()))
            }
            GeneralDataCommand::Set(general_data) => {
                manager
                    .apply(Operation::GeneralData(general_data.into()))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::InterrogationsPerWeekRangeIsEmpty => {
                            PyValueError::new_err("Interrogations per week range is empty")
                        }
                        UpdateError::WeekPatternsNeedTruncating(_week_patterns) => {
                            PyValueError::new_err("Some wwek patterns need truncating")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GeneralDataAnswer::Set)
            }
        }
    }

    fn execute_week_patterns_job<T: state::Manager>(
        week_patterns_command: &WeekPatternsCommand,
        manager: &mut T,
    ) -> PyResult<WeekPatternsAnswer> {
        match week_patterns_command {
            WeekPatternsCommand::GetAll => {
                let result = manager
                    .week_patterns_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, pattern)| (handle.into(), WeekPattern::from(pattern)))
                    .collect::<BTreeMap<_, _>>();

                Ok(WeekPatternsAnswer::GetAll(result))
            }
            WeekPatternsCommand::Get(handle) => {
                let result = manager
                    .week_patterns_get(handle.handle)
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(WeekPatternsAnswer::Get(result.into()))
            }
            WeekPatternsCommand::Create(pattern) => {
                let output = manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Create(pattern.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekNumberTooBig(_) => {
                            PyValueError::new_err("Week number larger than week_count")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::WeekPattern(handle) = output else {
                    panic!("No week pattern handle returned on WeekPatternsOperation::Create");
                };

                Ok(WeekPatternsAnswer::Create(handle.into()))
            }
            WeekPatternsCommand::Update(handle, pattern) => {
                manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Update(handle.handle, pattern.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekNumberTooBig(_) => {
                            PyValueError::new_err("Week number larger than week_count")
                        }
                        UpdateError::WeekPatternRemoved(_) => {
                            PyValueError::new_err("Week pattern was previsouly removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(WeekPatternsAnswer::Update)
            }
            WeekPatternsCommand::Remove(handle) => {
                manager
                    .apply(Operation::WeekPatterns(
                        state::WeekPatternsOperation::Remove(handle.handle),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::WeekPatternRemoved(_) => {
                            PyValueError::new_err("Week pattern was previsouly removed")
                        }
                        UpdateError::WeekPatternDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this week pattern",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(WeekPatternsAnswer::Remove)
            }
        }
    }

    fn execute_teachers_job<T: state::Manager>(
        teachers_command: &TeachersCommand,
        manager: &mut T,
    ) -> PyResult<TeachersAnswer> {
        match teachers_command {
            TeachersCommand::GetAll => {
                let result = manager
                    .teachers_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, teacher)| (handle.into(), Teacher::from(teacher)))
                    .collect::<BTreeMap<_, _>>();

                Ok(TeachersAnswer::GetAll(result))
            }
            TeachersCommand::Get(handle) => {
                let result = manager.teachers_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(TeachersAnswer::Get(result.into()))
            }
            TeachersCommand::Create(teacher) => {
                let output = manager
                    .apply(Operation::Teachers(state::TeachersOperation::Create(
                        teacher.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Teacher(handle) = output else {
                    panic!("No teacher handle returned on TeachersOperation::Create");
                };

                Ok(TeachersAnswer::Create(handle.into()))
            }
            TeachersCommand::Update(handle, teacher) => {
                manager
                    .apply(Operation::Teachers(state::TeachersOperation::Update(
                        handle.handle,
                        teacher.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TeacherRemoved(_) => {
                            PyValueError::new_err("Teacher was previsouly removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TeachersAnswer::Update)
            }
            TeachersCommand::Remove(handle) => {
                manager
                    .apply(Operation::Teachers(state::TeachersOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TeacherRemoved(_) => {
                            PyValueError::new_err("Teacher was previsouly removed")
                        }
                        UpdateError::TeacherDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this teacher",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TeachersAnswer::Remove)
            }
        }
    }

    fn execute_students_job<T: state::Manager>(
        students_command: &StudentsCommand,
        manager: &mut T,
    ) -> PyResult<StudentsAnswer> {
        match students_command {
            StudentsCommand::GetAll => {
                let result = manager
                    .students_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, student)| (handle.into(), Student::from(student)))
                    .collect::<BTreeMap<_, _>>();

                Ok(StudentsAnswer::GetAll(result))
            }
            StudentsCommand::Get(handle) => {
                let result = manager.students_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(StudentsAnswer::Get(result.into()))
            }
            StudentsCommand::Create(student) => {
                let output = manager
                    .apply(Operation::Students(state::StudentsOperation::Create(
                        student.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Student(handle) = output else {
                    panic!("No student handle returned on StudentsOperation::Create");
                };

                Ok(StudentsAnswer::Create(handle.into()))
            }
            StudentsCommand::Update(handle, student) => {
                manager
                    .apply(Operation::Students(state::StudentsOperation::Update(
                        handle.handle,
                        student.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(StudentsAnswer::Update)
            }
            StudentsCommand::Remove(handle) => {
                manager
                    .apply(Operation::Students(state::StudentsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        UpdateError::StudentDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this student",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(StudentsAnswer::Remove)
            }
        }
    }

    fn execute_subject_groups_job<T: state::Manager>(
        subject_groups_command: &SubjectGroupsCommand,
        manager: &mut T,
    ) -> PyResult<SubjectGroupsAnswer> {
        match subject_groups_command {
            SubjectGroupsCommand::GetAll => {
                let result = manager
                    .subject_groups_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, subject_group)| {
                        (handle.into(), SubjectGroup::from(subject_group))
                    })
                    .collect::<BTreeMap<_, _>>();

                Ok(SubjectGroupsAnswer::GetAll(result))
            }
            SubjectGroupsCommand::Get(handle) => {
                let result = manager
                    .subject_groups_get(handle.handle)
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(SubjectGroupsAnswer::Get(result.into()))
            }
            SubjectGroupsCommand::Create(subject_group) => {
                let output = manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Create(subject_group.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::SubjectGroup(handle) = output else {
                    panic!("No subject group handle returned on SubjectGroupsCommand::Create");
                };

                Ok(SubjectGroupsAnswer::Create(handle.into()))
            }
            SubjectGroupsCommand::Update(handle, subject_group) => {
                manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Update(handle.handle, subject_group.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectGroupRemoved(_) => {
                            PyValueError::new_err("Subject group was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectGroupsAnswer::Update)
            }
            SubjectGroupsCommand::Remove(handle) => {
                manager
                    .apply(Operation::SubjectGroups(
                        state::SubjectGroupsOperation::Remove(handle.handle),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectGroupRemoved(_) => {
                            PyValueError::new_err("Subject group was previously removed")
                        }
                        UpdateError::SubjectGroupDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this subject group",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectGroupsAnswer::Remove)
            }
        }
    }

    fn execute_incompats_job<T: state::Manager>(
        incompats_command: &IncompatsCommand,
        manager: &mut T,
    ) -> PyResult<IncompatsAnswer> {
        match incompats_command {
            IncompatsCommand::GetAll => {
                let result = manager
                    .incompats_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, incompat)| (handle.into(), Incompat::from(incompat)))
                    .collect::<BTreeMap<_, _>>();

                Ok(IncompatsAnswer::GetAll(result))
            }
            IncompatsCommand::Get(handle) => {
                let result = manager.incompats_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(IncompatsAnswer::Get(result.into()))
            }
            IncompatsCommand::Create(incompat) => {
                let output = manager
                    .apply(Operation::Incompats(state::IncompatsOperation::Create(
                        incompat.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::IncompatBadWeekPattern(week_pattern) => {
                            PyValueError::new_err(format!(
                                "Incompat references a bad week pattern handle {:?}",
                                week_pattern
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Incompat(handle) = output else {
                    panic!("No incompat handle returned on IncompatsCommand::Create");
                };

                Ok(IncompatsAnswer::Create(handle.into()))
            }
            IncompatsCommand::Update(handle, incompat) => {
                manager
                    .apply(Operation::Incompats(state::IncompatsOperation::Update(
                        handle.handle,
                        incompat.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::IncompatRemoved(_) => {
                            PyValueError::new_err("Incompat was previously removed")
                        }
                        UpdateError::IncompatBadWeekPattern(week_pattern) => {
                            PyValueError::new_err(format!(
                                "Incompat references a bad week pattern handle {:?}",
                                week_pattern
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(IncompatsAnswer::Update)
            }
            IncompatsCommand::Remove(handle) => {
                manager
                    .apply(Operation::Incompats(state::IncompatsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::IncompatRemoved(_) => {
                            PyValueError::new_err("Incompat was previously removed")
                        }
                        UpdateError::IncompatDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this incompat",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(IncompatsAnswer::Remove)
            }
        }
    }

    fn execute_group_lists_job<T: state::Manager>(
        group_lists_command: &GroupListsCommand,
        manager: &mut T,
    ) -> PyResult<GroupListsAnswer> {
        match group_lists_command {
            GroupListsCommand::GetAll => {
                let result = manager
                    .group_lists_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, group_list)| (handle.into(), GroupList::from(group_list)))
                    .collect::<BTreeMap<_, _>>();

                Ok(GroupListsAnswer::GetAll(result))
            }
            GroupListsCommand::Get(handle) => {
                let result = manager
                    .group_lists_get(handle.handle)
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(GroupListsAnswer::Get(result.into()))
            }
            GroupListsCommand::Create(group_list) => {
                let output = manager
                    .apply(Operation::GroupLists(state::GroupListsOperation::Create(
                        group_list.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupListBadStudent(student_handle) => {
                            PyValueError::new_err(format!(
                                "Group list references a bad student handle {:?}",
                                student_handle
                            ))
                        }
                        UpdateError::GroupListWithInconsistentStudentMapping => {
                            PyValueError::new_err("Inconsistent student mapping")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::GroupList(handle) = output else {
                    panic!("No group list handle returned on GroupListsCommand::Create");
                };

                Ok(GroupListsAnswer::Create(handle.into()))
            }
            GroupListsCommand::Update(handle, group_list) => {
                manager
                    .apply(Operation::GroupLists(state::GroupListsOperation::Update(
                        handle.handle,
                        group_list.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupListRemoved(_) => {
                            PyValueError::new_err("Group list was previously removed")
                        }
                        UpdateError::GroupListBadStudent(student_handle) => {
                            PyValueError::new_err(format!(
                                "Group list references a bad student handle {:?}",
                                student_handle
                            ))
                        }
                        UpdateError::GroupListWithInconsistentStudentMapping => {
                            PyValueError::new_err("Inconsistent student mapping")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupListsAnswer::Update)
            }
            GroupListsCommand::Remove(handle) => {
                manager
                    .apply(Operation::GroupLists(state::GroupListsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupListRemoved(_) => {
                            PyValueError::new_err("Group list was previously removed")
                        }
                        UpdateError::GroupListDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this group list",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupListsAnswer::Remove)
            }
        }
    }

    fn execute_subjects_job<T: state::Manager>(
        subjects_command: &SubjectsCommand,
        manager: &mut T,
    ) -> PyResult<SubjectsAnswer> {
        match subjects_command {
            SubjectsCommand::GetAll => {
                let result = manager
                    .subjects_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, subject)| (handle.into(), Subject::from(subject)))
                    .collect::<BTreeMap<_, _>>();

                Ok(SubjectsAnswer::GetAll(result))
            }
            SubjectsCommand::Get(handle) => {
                let result = manager.subjects_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(SubjectsAnswer::Get(result.into()))
            }
            SubjectsCommand::Create(group_list) => {
                let output = manager
                    .apply(Operation::Subjects(state::SubjectsOperation::Create(
                        group_list.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectBadSubjectGroup(subject_group_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                subject_group_handle
                            ))
                        }
                        UpdateError::SubjectBadIncompat(incompat_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                incompat_handle
                            ))
                        }
                        UpdateError::SubjectBadGroupList(group_list_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                group_list_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Subject(handle) = output else {
                    panic!("No subject handle returned on SubjectsCommand::Create");
                };

                Ok(SubjectsAnswer::Create(handle.into()))
            }
            SubjectsCommand::Update(handle, subject) => {
                manager
                    .apply(Operation::Subjects(state::SubjectsOperation::Update(
                        handle.handle,
                        subject.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectRemoved(_) => {
                            PyValueError::new_err("Subject was previously removed")
                        }
                        UpdateError::SubjectBadSubjectGroup(subject_group_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                subject_group_handle
                            ))
                        }
                        UpdateError::SubjectBadIncompat(incompat_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                incompat_handle
                            ))
                        }
                        UpdateError::SubjectBadGroupList(group_list_handle) => {
                            PyValueError::new_err(format!(
                                "Subject references a bad subject group handle {:?}",
                                group_list_handle
                            ))
                        }
                        UpdateError::SubjectWithStudentRegistered(student_handle) => {
                            PyValueError::new_err(format!(
                                "Cannot change subject group: student {:?} is still resgistered",
                                student_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectsAnswer::Update)
            }
            SubjectsCommand::Remove(handle) => {
                manager
                    .apply(Operation::Subjects(state::SubjectsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SubjectRemoved(_) => {
                            PyValueError::new_err("Subject was previously removed")
                        }
                        UpdateError::SubjectDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this subject",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SubjectsAnswer::Remove)
            }
        }
    }

    fn execute_time_slots_job<T: state::Manager>(
        time_slots_command: &TimeSlotsCommand,
        manager: &mut T,
    ) -> PyResult<TimeSlotsAnswer> {
        match time_slots_command {
            TimeSlotsCommand::GetAll => {
                let result = manager
                    .time_slots_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, time_slot)| (handle.into(), TimeSlot::from(time_slot)))
                    .collect::<BTreeMap<_, _>>();

                Ok(TimeSlotsAnswer::GetAll(result))
            }
            TimeSlotsCommand::Get(handle) => {
                let result = manager.time_slots_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(TimeSlotsAnswer::Get(result.into()))
            }
            TimeSlotsCommand::Create(time_slot) => {
                let output = manager
                    .apply(Operation::TimeSlots(state::TimeSlotsOperation::Create(
                        time_slot.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TimeSlotBadSubject(subject_group_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad subject group handle {:?}",
                                subject_group_handle
                            ))
                        }
                        UpdateError::TimeSlotBadTeacher(teacher_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad teacher handle {:?}",
                                teacher_handle
                            ))
                        }
                        UpdateError::TimeSlotBadWeekPattern(week_pattern_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad week pattern handle {:?}",
                                week_pattern_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::TimeSlot(handle) = output else {
                    panic!("No time slot handle returned on TimeSlotsCommand::Create");
                };

                Ok(TimeSlotsAnswer::Create(handle.into()))
            }
            TimeSlotsCommand::Update(handle, time_slot) => {
                manager
                    .apply(Operation::TimeSlots(state::TimeSlotsOperation::Update(
                        handle.handle,
                        time_slot.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TimeSlotRemoved(_) => {
                            PyValueError::new_err("Time slot was previously removed")
                        }
                        UpdateError::TimeSlotBadSubject(subject_group_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad subject group handle {:?}",
                                subject_group_handle
                            ))
                        }
                        UpdateError::TimeSlotBadTeacher(teacher_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad teacher handle {:?}",
                                teacher_handle
                            ))
                        }
                        UpdateError::TimeSlotBadWeekPattern(week_pattern_handle) => {
                            PyValueError::new_err(format!(
                                "Time slot references a bad week pattern handle {:?}",
                                week_pattern_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TimeSlotsAnswer::Update)
            }
            TimeSlotsCommand::Remove(handle) => {
                manager
                    .apply(Operation::TimeSlots(state::TimeSlotsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::TimeSlotRemoved(_) => {
                            PyValueError::new_err("Time slot was previously removed")
                        }
                        UpdateError::TimeSlotDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this time slot",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(TimeSlotsAnswer::Remove)
            }
        }
    }

    fn execute_groupings_job<T: state::Manager>(
        groupings_command: &GroupingsCommand,
        manager: &mut T,
    ) -> PyResult<GroupingsAnswer> {
        match groupings_command {
            GroupingsCommand::GetAll => {
                let result = manager
                    .groupings_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, grouping)| (handle.into(), Grouping::from(grouping)))
                    .collect::<BTreeMap<_, _>>();

                Ok(GroupingsAnswer::GetAll(result))
            }
            GroupingsCommand::Get(handle) => {
                let result = manager.groupings_get(handle.handle).map_err(|e| match e {
                    IdError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                    IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                })?;

                Ok(GroupingsAnswer::Get(result.into()))
            }
            GroupingsCommand::Create(grouping) => {
                let output = manager
                    .apply(Operation::Groupings(state::GroupingsOperation::Create(
                        grouping.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingBadTimeSlot(time_slot_handle) => {
                            PyValueError::new_err(format!(
                                "Grouping references a bad time slot handle {:?}",
                                time_slot_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::Grouping(handle) = output else {
                    panic!("No grouping handle returned on GroupingsCommand::Create");
                };

                Ok(GroupingsAnswer::Create(handle.into()))
            }
            GroupingsCommand::Update(handle, grouping) => {
                manager
                    .apply(Operation::Groupings(state::GroupingsOperation::Update(
                        handle.handle,
                        grouping.into(),
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingRemoved(_) => {
                            PyValueError::new_err("Grouping was previously removed")
                        }
                        UpdateError::GroupingBadTimeSlot(time_slot_handle) => {
                            PyValueError::new_err(format!(
                                "Grouping references a bad time slot handle {:?}",
                                time_slot_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupingsAnswer::Update)
            }
            GroupingsCommand::Remove(handle) => {
                manager
                    .apply(Operation::Groupings(state::GroupingsOperation::Remove(
                        handle.handle,
                    )))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingRemoved(_) => {
                            PyValueError::new_err("Grouping was previously removed")
                        }
                        UpdateError::GroupingDependanciesRemaining(_) => PyValueError::new_err(
                            "There are remaining dependancies on this grouping",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupingsAnswer::Remove)
            }
        }
    }

    fn execute_grouping_incompats_job<T: state::Manager>(
        grouping_incompats_command: &GroupingIncompatsCommand,
        manager: &mut T,
    ) -> PyResult<GroupingIncompatsAnswer> {
        match grouping_incompats_command {
            GroupingIncompatsCommand::GetAll => {
                let result = manager
                    .grouping_incompats_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, grouping_incompat)| {
                        (handle.into(), GroupingIncompat::from(grouping_incompat))
                    })
                    .collect::<BTreeMap<_, _>>();

                Ok(GroupingIncompatsAnswer::GetAll(result))
            }
            GroupingIncompatsCommand::Get(handle) => {
                let result =
                    manager
                        .grouping_incompats_get(handle.handle)
                        .map_err(|e| match e {
                            IdError::InternalError(int_err) => {
                                PyException::new_err(int_err.to_string())
                            }
                            IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                        })?;

                Ok(GroupingIncompatsAnswer::Get(result.into()))
            }
            GroupingIncompatsCommand::Create(grouping_incompat) => {
                let output = manager
                    .apply(Operation::GroupingIncompats(
                        state::GroupingIncompatsOperation::Create(grouping_incompat.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingIncompatBadGrouping(grouping_handle) => {
                            PyValueError::new_err(format!(
                                "Grouping incompat references a bad grouping handle {:?}",
                                grouping_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::GroupingIncompat(handle) = output else {
                    panic!("No grouping incompat handle returned on GroupingsCommand::Create");
                };

                Ok(GroupingIncompatsAnswer::Create(handle.into()))
            }
            GroupingIncompatsCommand::Update(handle, grouping) => {
                manager
                    .apply(Operation::GroupingIncompats(
                        state::GroupingIncompatsOperation::Update(handle.handle, grouping.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingIncompatRemoved(_) => {
                            PyValueError::new_err("Grouping incompat was previously removed")
                        }
                        UpdateError::GroupingIncompatBadGrouping(grouping_handle) => {
                            PyValueError::new_err(format!(
                                "Grouping incompat references a bad grouping handle {:?}",
                                grouping_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupingIncompatsAnswer::Update)
            }
            GroupingIncompatsCommand::Remove(handle) => {
                manager
                    .apply(Operation::GroupingIncompats(
                        state::GroupingIncompatsOperation::Remove(handle.handle),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::GroupingIncompatRemoved(_) => {
                            PyValueError::new_err("Grouping incompat was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(GroupingIncompatsAnswer::Remove)
            }
        }
    }

    fn execute_register_student_job<T: state::Manager>(
        register_student_command: &RegisterStudentCommand,
        manager: &mut T,
    ) -> PyResult<RegisterStudentAnswer> {
        match register_student_command {
            RegisterStudentCommand::InSubjectGroupGet(student_handle, subject_group_handle) => {
                let result = manager
                    .subject_group_for_student_get(
                        student_handle.handle,
                        subject_group_handle.handle,
                    )
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        Id2Error::InvalidId1(_student_id) => {
                            PyValueError::new_err("Invalid student handle")
                        }
                        Id2Error::InvalidId2(_subject_group_id) => {
                            PyValueError::new_err("Invalid subject group handle")
                        }
                    })?;

                Ok(RegisterStudentAnswer::InSubjectGroupGet(
                    result.map(|x| x.into()),
                ))
            }
            RegisterStudentCommand::InSubjectGroupSet(
                student_handle,
                subject_group_handle,
                subject_handle,
            ) => {
                manager
                    .apply(Operation::RegisterStudent(
                        state::RegisterStudentOperation::InSubjectGroup(
                            student_handle.handle,
                            subject_group_handle.handle,
                            subject_handle.clone().map(|x| x.handle),
                        ),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        UpdateError::SubjectGroupRemoved(_) => {
                            PyValueError::new_err("Subject group was previously removed")
                        }
                        UpdateError::SubjectRemoved(_) => {
                            PyValueError::new_err("Subject was previously removed")
                        }
                        UpdateError::RegisterStudentBadSubject(_, _) => PyValueError::new_err(
                            "Subject is not a valid subject for the given subject group",
                        ),
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(RegisterStudentAnswer::InSubjectGroupSet)
            }
            RegisterStudentCommand::InIncompatGet(student_handle, incompat_handle) => {
                let result = manager
                    .incompat_for_student_get(student_handle.handle, incompat_handle.handle)
                    .map_err(|e| match e {
                        Id2Error::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        Id2Error::InvalidId1(_student_id) => {
                            PyValueError::new_err("Invalid student handle")
                        }
                        Id2Error::InvalidId2(_incompat_id) => {
                            PyValueError::new_err("Invalid incompat handle")
                        }
                    })?;

                Ok(RegisterStudentAnswer::InIncompatGet(result))
            }
            RegisterStudentCommand::InIncompatSet(student_handle, incompat_handle, enabled) => {
                manager
                    .apply(Operation::RegisterStudent(
                        state::RegisterStudentOperation::InIncompat(
                            student_handle.handle,
                            incompat_handle.handle,
                            *enabled,
                        ),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::StudentRemoved(_) => {
                            PyValueError::new_err("Student was previously removed")
                        }
                        UpdateError::IncompatRemoved(_) => {
                            PyValueError::new_err("Incompat was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(RegisterStudentAnswer::InIncompatSet)
            }
        }
    }

    fn execute_slot_selections_job<T: state::Manager>(
        slot_selections_command: &SlotSelectionsCommand,
        manager: &mut T,
    ) -> PyResult<SlotSelectionsAnswer> {
        match slot_selections_command {
            SlotSelectionsCommand::GetAll => {
                let result = manager
                    .slot_selections_get_all()
                    .map_err(|e| PyException::new_err(e.to_string()))?
                    .into_iter()
                    .map(|(handle, slot_selection)| {
                        (handle.into(), SlotSelection::from(slot_selection))
                    })
                    .collect::<BTreeMap<_, _>>();

                Ok(SlotSelectionsAnswer::GetAll(result))
            }
            SlotSelectionsCommand::Get(handle) => {
                let result = manager
                    .slot_selections_get(handle.handle)
                    .map_err(|e| match e {
                        IdError::InternalError(int_err) => {
                            PyException::new_err(int_err.to_string())
                        }
                        IdError::InvalidId(_) => PyValueError::new_err("Invalid handle"),
                    })?;

                Ok(SlotSelectionsAnswer::Get(result.into()))
            }
            SlotSelectionsCommand::Create(slot_selection) => {
                let output = manager
                    .apply(Operation::SlotSelections(
                        state::SlotSelectionsOperation::Create(slot_selection.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SlotSelectionBadSubject(subject_handle) => {
                            PyValueError::new_err(format!(
                                "Slot Selection references a bad subject handle {:?}",
                                subject_handle
                            ))
                        }
                        UpdateError::SlotSelectionBadTimeSlot(time_slot_handle) => {
                            PyValueError::new_err(format!(
                                "Slot Selection references a bad time slot handle {:?} (it might be valid but refer to another subject)",
                                time_slot_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                let ReturnHandle::SlotSelection(handle) = output else {
                    panic!("No slot selection handle returned on SlotSelectionsCommand::Create");
                };

                Ok(SlotSelectionsAnswer::Create(handle.into()))
            }
            SlotSelectionsCommand::Update(handle, slot_selection) => {
                manager
                    .apply(Operation::SlotSelections(
                        state::SlotSelectionsOperation::Update(handle.handle, slot_selection.into()),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SlotSelectionRemoved(_) => {
                            PyValueError::new_err("Slot selection was previously removed")
                        }
                        UpdateError::SlotSelectionBadSubject(subject_handle) => {
                            PyValueError::new_err(format!(
                                "Slot Selection references a bad subject handle {:?}",
                                subject_handle
                            ))
                        }
                        UpdateError::SlotSelectionBadTimeSlot(time_slot_handle) => {
                            PyValueError::new_err(format!(
                                "Slot Selection references a bad time slot handle {:?} (it might be valid but refer to another subject)",
                                time_slot_handle
                            ))
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SlotSelectionsAnswer::Update)
            }
            SlotSelectionsCommand::Remove(handle) => {
                manager
                    .apply(Operation::SlotSelections(
                        state::SlotSelectionsOperation::Remove(handle.handle),
                    ))
                    .map_err(|e| match e {
                        UpdateError::Internal(int_err) => PyException::new_err(int_err.to_string()),
                        UpdateError::SlotSelectionRemoved(_) => {
                            PyValueError::new_err("Slot selection was previously removed")
                        }
                        _ => panic!("Unexpected error!"),
                    })?;

                Ok(SlotSelectionsAnswer::Remove)
            }
        }
    }

    fn execute_job<T: state::Manager>(command: &Command, manager: &mut T) -> PyResult<Answer> {
        match command {
            Command::GeneralData(general_data_command) => {
                let answer = Self::execute_general_data_job(general_data_command, manager)?;
                Ok(Answer::GeneralData(answer))
            }
            Command::WeekPatterns(week_patterns_command) => {
                let answer = Self::execute_week_patterns_job(week_patterns_command, manager)?;
                Ok(Answer::WeekPatterns(answer))
            }
            Command::Teachers(teachers_command) => {
                let answer = Self::execute_teachers_job(teachers_command, manager)?;
                Ok(Answer::Teachers(answer))
            }
            Command::Students(students_command) => {
                let answer = Self::execute_students_job(students_command, manager)?;
                Ok(Answer::Students(answer))
            }
            Command::SubjectGroups(subject_groups_command) => {
                let answer = Self::execute_subject_groups_job(subject_groups_command, manager)?;
                Ok(Answer::SubjectGroups(answer))
            }
            Command::Incompats(incompats_command) => {
                let answer = Self::execute_incompats_job(incompats_command, manager)?;
                Ok(Answer::Incompats(answer))
            }
            Command::GroupLists(group_lists_command) => {
                let answer = Self::execute_group_lists_job(group_lists_command, manager)?;
                Ok(Answer::GroupLists(answer))
            }
            Command::Subjects(subjects_command) => {
                let answer = Self::execute_subjects_job(subjects_command, manager)?;
                Ok(Answer::Subjects(answer))
            }
            Command::TimeSlots(time_slots_command) => {
                let answer = Self::execute_time_slots_job(time_slots_command, manager)?;
                Ok(Answer::TimeSlots(answer))
            }
            Command::Groupings(groupings_command) => {
                let answer = Self::execute_groupings_job(groupings_command, manager)?;
                Ok(Answer::Groupings(answer))
            }
            Command::GroupingIncompats(grouping_incompats_command) => {
                let answer =
                    Self::execute_grouping_incompats_job(grouping_incompats_command, manager)?;
                Ok(Answer::GroupingIncompats(answer))
            }
            Command::RegisterStudent(register_student_command) => {
                let answer = Self::execute_register_student_job(register_student_command, manager)?;
                Ok(Answer::RegisterStudent(answer))
            }
            Command::SlotSelections(slot_selections_command) => {
                let answer = Self::execute_slot_selections_job(slot_selections_command, manager)?;
                Ok(Answer::SlotSelections(answer))
            }
            Command::Undo => {
                manager.undo().map_err(|e| match e {
                    UndoError::HistoryDepleted => PyException::new_err("History depleted"),
                    UndoError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                })?;

                Ok(Answer::Undo)
            }
            Command::Redo => {
                manager.redo().map_err(|e| match e {
                    RedoError::HistoryFullyRewounded => {
                        PyException::new_err("History fully rewounded")
                    }
                    RedoError::InternalError(int_err) => PyException::new_err(int_err.to_string()),
                })?;

                Ok(Answer::Redo)
            }
            Command::Exit => panic!("Exit command should be treated on level above"),
        }
    }

    fn send_command_internal(sender: &Sender<Job>, command: Command) -> Receiver<PyResult<Answer>> {
        let (answer_sender, answer_receiver) = mpsc::channel();

        let job = Job {
            command,
            answer: answer_sender,
        };

        sender
            .send(job)
            .expect("Python code should have finished before worker thread.");

        answer_receiver
    }

    fn send_command(py: Python, sender: &Sender<Job>, command: Command) -> PyResult<Answer> {
        let receiver = Self::send_command_internal(sender, command);

        py.allow_threads(move || receiver.recv().unwrap())
    }
}
