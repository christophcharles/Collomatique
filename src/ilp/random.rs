pub trait RandomGen {
    fn randbool(&mut self) -> bool;
    fn random(&mut self) -> f64;
    fn rand_elem<T: Clone>(&mut self, elems: &[T]) -> T;
}

pub struct DefaultRndGen {
    thread_rng: rand::rngs::ThreadRng,
}

impl DefaultRndGen {
    pub fn new() -> Self {
        DefaultRndGen {
            thread_rng: rand::thread_rng(),
        }
    }
}

impl RandomGen for DefaultRndGen {
    fn randbool(&mut self) -> bool {
        use rand::Rng;
        self.thread_rng.gen_bool(0.5)
    }

    fn random(&mut self) -> f64 {
        use rand::Rng;
        self.thread_rng.gen::<f64>()
    }

    fn rand_elem<T: Clone>(&mut self, elems: &[T]) -> T {
        use rand::Rng;
        let i = self.thread_rng.gen_range(0..elems.len());
        elems[i].clone()
    }
}
