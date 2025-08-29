pub trait RandomGen: Clone + Send + Sync {
    fn randbool(&self) -> bool;
    fn random(&self) -> f64;
    fn rand_in_range(&self, range: std::ops::Range<usize>) -> usize;
    fn rand_elem<T: Clone>(&self, elems: &[T]) -> T {
        let i = self.rand_in_range(0..elems.len());
        elems[i].clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DefaultRndGen {}

impl DefaultRndGen {
    pub fn new() -> Self {
        DefaultRndGen {}
    }
}

impl RandomGen for DefaultRndGen {
    fn randbool(&self) -> bool {
        use rand::Rng;
        let mut thread_rng = rand::thread_rng();
        thread_rng.gen_bool(0.5)
    }

    fn random(&self) -> f64 {
        use rand::Rng;
        let mut thread_rng = rand::thread_rng();
        thread_rng.gen::<f64>()
    }

    fn rand_in_range(&self, range: std::ops::Range<usize>) -> usize {
        use rand::Rng;
        let mut thread_rng = rand::thread_rng();
        thread_rng.gen_range(range)
    }
}
