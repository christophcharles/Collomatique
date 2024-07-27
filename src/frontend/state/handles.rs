#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekPatternHandle(usize);

pub(super) trait Handle: Send + Sync + Clone + Copy {
    fn new(value: usize) -> Self;
    fn get(self) -> usize;
}

impl Handle for WeekPatternHandle {
    fn new(value: usize) -> Self {
        WeekPatternHandle(value)
    }
    fn get(self) -> usize {
        self.0
    }
}

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
}

impl<T: backend::Storage> ManagerCollection<T> {
    pub(super) fn new() -> Self {
        ManagerCollection {
            week_patterns: Manager::new(),
        }
    }
}
