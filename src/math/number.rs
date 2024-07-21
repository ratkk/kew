pub trait Zero {
    fn zero() -> Self;
}

impl Zero for usize {
    fn zero() -> Self {
        0
    }
}
