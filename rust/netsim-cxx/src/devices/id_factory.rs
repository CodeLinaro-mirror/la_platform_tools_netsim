/// A factory for generating typed identifiers.
///
use std::ops::Add;
pub struct IdFactory<T>
where
    T: Add<Output = T> + Copy,
{
    next_id: T,
    increment: T,
}

impl<T> IdFactory<T>
where
    T: Add<Output = T> + Copy,
{
    pub fn new(start_id: T, increment: T) -> Self {
        Self { next_id: start_id, increment }
    }
    pub fn next_id(&mut self) -> T {
        let id = self.next_id;
        self.next_id = self.next_id + self.increment;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::RwLock;
    type BlueIdentifier = u32;

    #[test]
    fn test_blue_id_factory() {
        let ids: RwLock<IdFactory<BlueIdentifier>> = RwLock::new(IdFactory::new(1000, 1));
        assert_eq!(ids.write().unwrap().next_id(), 1000);
        assert_eq!(ids.write().unwrap().next_id(), 1001);
        assert_eq!(ids.write().unwrap().next_id(), 1002);
    }
}
