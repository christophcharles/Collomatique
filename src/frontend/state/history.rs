use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedOperation {
    General(AnnotatedGeneralOperation),
    WeekPatterns(AnnotatedWeekPatternsOperation),
    Aggregated(Vec<AnnotatedOperation>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedGeneralOperation {
    SetWeekCount(NonZeroU32),
    SetMaxInterrogationsPerDay(Option<NonZeroU32>),
    SetInterrogationsPerWeekRange(Option<std::ops::Range<u32>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotatedWeekPatternsOperation {
    Create(handles::WeekPatternHandle, backend::WeekPattern),
    Remove(handles::WeekPatternHandle),
    Update(handles::WeekPatternHandle, backend::WeekPattern),
}

impl AnnotatedGeneralOperation {
    fn annotate(op: GeneralOperation) -> Self {
        match op {
            GeneralOperation::SetWeekCount(week_count) => {
                AnnotatedGeneralOperation::SetWeekCount(week_count)
            }
            GeneralOperation::SetMaxInterrogationsPerDay(max_int_per_day) => {
                AnnotatedGeneralOperation::SetMaxInterrogationsPerDay(max_int_per_day)
            }
            GeneralOperation::SetInterrogationsPerWeekRange(int_per_week) => {
                AnnotatedGeneralOperation::SetInterrogationsPerWeekRange(int_per_week)
            }
        }
    }
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

impl AnnotatedOperation {
    pub fn annotate<T: backend::Storage>(
        op: Operation,
        handle_managers: &mut handles::ManagerCollection<T>,
    ) -> Self {
        match op {
            Operation::General(op) => {
                AnnotatedOperation::General(AnnotatedGeneralOperation::annotate(op))
            }
            Operation::WeekPatterns(op) => AnnotatedOperation::WeekPatterns(
                AnnotatedWeekPatternsOperation::annotate(op, handle_managers),
            ),
            Operation::Aggregated(ops) => AnnotatedOperation::Aggregated(
                ops.into_iter()
                    .map(|op| AnnotatedOperation::annotate(op, handle_managers))
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReversibleOperation {
    pub forward: AnnotatedOperation,
    pub backward: AnnotatedOperation,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ModificationHistory {
    history: std::collections::VecDeque<ReversibleOperation>,
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

    pub fn apply(&mut self, reversible_op: ReversibleOperation) {
        self.history.truncate(self.history_pointer);

        self.history_pointer += 1;
        self.history.push_back(reversible_op);

        self.truncate_history_as_needed();
    }

    pub fn can_undo(&self) -> bool {
        self.history_pointer > 0
    }

    pub fn can_redo(&self) -> bool {
        self.history_pointer < self.history.len()
    }

    pub fn undo(&mut self) -> Option<AnnotatedOperation> {
        if !self.can_undo() {
            return None;
        }

        self.history_pointer -= 1;

        assert!(self.history_pointer < self.history.len());

        let last_op = self.history[self.history_pointer].clone();

        Some(last_op.backward)
    }

    pub fn redo(&mut self) -> Option<AnnotatedOperation> {
        if !self.can_redo() {
            return None;
        }

        let new_op = self.history[self.history_pointer].clone();
        self.history_pointer += 1;

        Some(new_op.forward)
    }
}
