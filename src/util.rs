#[derive(Debug, Copy, Clone)]
pub struct Addr(pub u32);

impl Default for Addr {
    fn default() -> Self {
        Addr(0)
    }
}
