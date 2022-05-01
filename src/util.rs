use associative_cache::Capacity;

#[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq)]
pub struct Addr(pub u32);

impl Addr {
    pub fn to_cache_line(self) -> Addr {
        Addr(self.0 & !(64 - 1))
    }
}

#[derive(Debug)]
pub struct CacheCapacity<const BYTES: usize>;

impl<const BYTES: usize> Capacity for CacheCapacity<{ BYTES }> {
    const CAPACITY: usize = BYTES / 64;
}
