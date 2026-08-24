use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};

/// Replaces a factory's whole content.
///
/// For short, cheap lists where rebuilding beats diffing — unlike
/// [update_vec_deque] it sends no update message, so it also suits components
/// whose `Input` carries nothing.
pub fn refill_vec_deque<C: FactoryComponent<Index = DynamicIndex>>(
    factory: &mut FactoryVecDeque<C>,
    items: impl IntoIterator<Item = C::Init>,
) {
    let mut guard = factory.guard();
    guard.clear();
    for item in items {
        guard.push_back(item);
    }
}

pub fn update_vec_deque<C: FactoryComponent<Index = DynamicIndex>>(
    factory: &mut FactoryVecDeque<C>,
    iterator: impl ExactSizeIterator<Item = C::Init>,
    update_fn: impl Fn(C::Init) -> C::Input,
) {
    let new_len = iterator.len();
    let is_empty = new_len == 0;

    let mut guard = factory.guard();
    if is_empty {
        guard.clear();
    } else {
        let current_len = guard.len();
        if current_len > new_len {
            for _i in new_len..current_len {
                guard.pop_back();
            }
        }

        let current_len = guard.len();
        for (i, item) in iterator.enumerate() {
            if i < current_len {
                guard.send(i, update_fn(item));
            } else {
                guard.push_back(item);
            }
        }
    }
    guard.drop()
}
