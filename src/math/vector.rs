use super::number::Zero;

pub struct Vector<T, const U: usize> {
    data: [T; U],
}

impl<T, const U: usize> Vector<T, U>
where
    T: Zero + Copy,
{
    pub fn new() -> Self {
        Self {
            data: [T::zero(); U],
        }
    }
}
