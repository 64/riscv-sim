use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BranchTargetBuffer {
    cache: HashMap<u32, u32>, // PC -> Addr
    capacity: usize,
}

#[derive(Debug, Clone)]
pub struct BranchPredictor;

impl BranchTargetBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            capacity,
        }
    }

    pub fn get(&self, pc: u32) -> Option<u32> {
        self.cache.get(&pc).copied()
    }

    pub fn add_entry(&mut self, pc: u32, target: u32) {
        self.cache.insert(pc, target);
        assert!(self.cache.len() < self.capacity);
    }
}

impl BranchPredictor {
    pub fn new() -> Self { Self }

    pub fn predict_taken(&self, pc: u32, target: u32) -> bool {
        // BTFNT
        target < pc
    }
}
