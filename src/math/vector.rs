use super::number::Zero;

#[derive(Clone, Copy)]
pub struct Vector<T, const U: usize> {
    data: [T; U],
}

impl<T, const U: usize> From<[T; U]> for Vector<T, U> {
    fn from(data: [T; U]) -> Self {
        Self { data }
    }
}

impl<T, const U: usize> Default for Vector<T, U>
where
    T: Zero + Copy,
{
    fn default() -> Self {
        Self {
            data: [T::zero(); U],
        }
    }
}
