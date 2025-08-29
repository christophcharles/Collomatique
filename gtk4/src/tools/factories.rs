use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};

pub fn update_vec_deque<'a, C: FactoryComponent<Index = DynamicIndex>>(
    factory: &'a mut FactoryVecDeque<C>,
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
