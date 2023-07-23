pub use rand::rngs::ThreadRng;
pub use rand::Rng;

pub fn new_rng() -> ThreadRng {
    rand::thread_rng()
}
