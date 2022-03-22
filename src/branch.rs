// use lru_mem::LruCache;

// #[derive(Debug, Clone)]
// pub struct BranchTargetBuffer {
//     cache: LruCache<u32, u32>, // PC -> Addr
// }

#[derive(Debug, Clone)]
pub struct BranchPredictor;

// impl BranchTargetBuffer {
//     pub fn new(capacity: usize) -> Self {
//         Self {
//             cache: LruCache::new(capacity),
//         }
//     }

//     pub fn get(&mut self, pc: u32) -> Option<u32> {
//         self.cache.get(&pc).copied()
//     }

//     pub fn add_entry(&mut self, pc: u32, target: u32) {
//         self.cache.insert(pc, target).unwrap();
//     }
// }

impl BranchPredictor {
    pub fn new() -> Self {
        Self
    }

    pub fn predict_taken(&self, pc: u32, target: u32) -> bool {
        // BTFNT
        target < pc
        // false
        // true
    }
}
