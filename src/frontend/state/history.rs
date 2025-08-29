use std::collections::VecDeque;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedOperation {
    GeneralData(backend::GeneralData),
    WeekPatterns(AnnotatedWeekPatternsOperation),
    Teachers(AnnotatedTeachersOperation),
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
            TeachersOperation::Create(pattern) => {
                let handle = handle_managers.teachers.create_handle();
                AnnotatedTeachersOperation::Create(handle, pattern)
            }
            TeachersOperation::Remove(handle) => AnnotatedTeachersOperation::Remove(handle),
            TeachersOperation::Update(handle, pattern) => {
                AnnotatedTeachersOperation::Update(handle, pattern)
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
