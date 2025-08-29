#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WeekPatternHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TeacherHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudentHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectGroupHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncompatHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupListHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeSlotHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupingHandle(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupingIncompatHandle(usize);

pub(super) trait Handle: Send + Sync + Clone + Copy {
    fn new(value: usize) -> Self;
    fn get(self) -> usize;
}

macro_rules! impl_handle {
    ($HandleType:ident) => {
        impl Handle for $HandleType {
            fn new(value: usize) -> Self {
                $HandleType(value)
            }
            fn get(self) -> usize {
                self.0
            }
        }
    };
}

impl_handle!(WeekPatternHandle);
impl_handle!(TeacherHandle);
impl_handle!(StudentHandle);
impl_handle!(SubjectGroupHandle);
impl_handle!(IncompatHandle);
impl_handle!(GroupListHandle);
impl_handle!(SubjectHandle);
impl_handle!(TimeSlotHandle);
impl_handle!(GroupingHandle);
impl_handle!(GroupingIncompatHandle);

use crate::backend;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct Manager<Id: backend::OrdId, H: Handle> {
    id_to_handle_map: BTreeMap<Id, H>,
    handle_to_id_map: Vec<Option<Id>>,
}

impl<Id: backend::OrdId, H: Handle> Manager<Id, H> {
    fn new() -> Self {
        Manager {
            id_to_handle_map: BTreeMap::new(),
            handle_to_id_map: Vec::new(),
        }
    }

    pub(super) fn get_handle(&mut self, id: Id) -> H {
        if let Some(&handle) = self.id_to_handle_map.get(&id) {
            return handle;
        }

        let new_handle = H::new(self.handle_to_id_map.len());

        self.id_to_handle_map.insert(id, new_handle);
        self.handle_to_id_map.push(Some(id));

        new_handle
    }

    pub(super) fn get_id(&self, handle: H) -> Option<Id> {
        self.handle_to_id_map
            .get(handle.get())
            .copied()
            .expect("Id requests should be done on valid handles")
    }

    pub(super) fn update_handle(&mut self, handle: H, new_id: Option<Id>) {
        let old_id_opt = self
            .handle_to_id_map
            .get_mut(handle.get())
            .expect("handles that need updating should be valid");

        if let Some(old_id) = *old_id_opt {
            self.id_to_handle_map.remove(&old_id);
        }

        *old_id_opt = new_id;
        if let Some(id) = new_id {
            self.id_to_handle_map.insert(id, handle);
        }
    }

    pub(super) fn create_handle(&mut self) -> H {
        let new_handle = H::new(self.handle_to_id_map.len());

        self.handle_to_id_map.push(None);

        new_handle
    }
}

#[derive(Debug)]
pub struct ManagerCollection<T: backend::Storage> {
    pub(super) week_patterns: Manager<T::WeekPatternId, WeekPatternHandle>,
    pub(super) teachers: Manager<T::TeacherId, TeacherHandle>,
    pub(super) students: Manager<T::StudentId, StudentHandle>,
    pub(super) subject_groups: Manager<T::SubjectGroupId, SubjectGroupHandle>,
    pub(super) incompats: Manager<T::IncompatId, IncompatHandle>,
    pub(super) group_lists: Manager<T::GroupListId, GroupListHandle>,
    pub(super) subjects: Manager<T::SubjectId, SubjectHandle>,
    pub(super) time_slots: Manager<T::TimeSlotId, TimeSlotHandle>,
    pub(super) groupings: Manager<T::GroupingId, GroupingHandle>,
    pub(super) grouping_incompats: Manager<T::GroupingIncompatId, GroupingIncompatHandle>,
}

impl<T: backend::Storage> ManagerCollection<T> {
    pub(super) fn new() -> Self {
        ManagerCollection {
            week_patterns: Manager::new(),
            teachers: Manager::new(),
            students: Manager::new(),
            subject_groups: Manager::new(),
            incompats: Manager::new(),
            group_lists: Manager::new(),
            subjects: Manager::new(),
            time_slots: Manager::new(),
            groupings: Manager::new(),
            grouping_incompats: Manager::new(),
        }
    }
}
