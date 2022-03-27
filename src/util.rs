#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Addr(pub u32);

impl Default for Addr {
    fn default() -> Self {
        Addr(0)
    }
}

impl Addr {
    pub fn to_cache_line(&self) -> Addr {
        Addr(self.0 & !(64 - 1))
    }
}
