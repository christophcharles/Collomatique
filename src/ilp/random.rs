pub trait RandomGen {
    fn randbool(&mut self) -> bool;
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
}
