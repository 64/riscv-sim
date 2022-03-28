use crate::{inst::Tag, util::Addr};
use associative_cache::*;

const L1_CAPACITY_BYTES: usize = 16_000;
const L2_CAPACITY_BYTES: usize = 32_000;
const L3_CAPACITY_BYTES: usize = 128_000;
const DRAM_CAPACITY_BYTES: usize = 256_000;

pub const STACK_TOP: usize = DRAM_CAPACITY_BYTES - 2_000;

const L1_LATENCY: u64 = 5;
const L2_LATENCY: u64 = 20;
const L3_LATENCY: u64 = 40;
const DRAM_LATENCY: u64 = 400;
// const DRAM_LATENCY: u64 = 1;

#[derive(Debug, Clone, Default)]
pub struct MainMemory {
    mem: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Pending {
    tag: Tag,
    current: u64,
    end: u64,
}

#[derive(Debug)]
pub struct MemoryHierarchy {
    pub main: MainMemory,
    l1: L1Cache,
    l2: L2Cache,
    l3: L3Cache,
    pending_fetches: Vec<Pending>,
}

impl MemoryHierarchy {
    pub fn new(mem: MainMemory) -> Self {
        Self {
            main: mem,
            l1: AssociativeCache::default(),
            l2: AssociativeCache::default(),
            l3: AssociativeCache::default(),
            pending_fetches: Default::default(),
        }
    }

    pub fn access_complete(&mut self, tag: Tag, addr: Addr) -> bool {
        match self.pending_fetches.iter().find(|p| p.tag == tag) {
            Some(p) => p.current >= p.end,
            None => {
                let addr = addr.to_cache_line();
                let latency = if self.l1.get(&addr).is_some() {
                    L1_LATENCY
                } else if self.l2.get(&addr).is_some() {
                    L2_LATENCY
                } else if self.l3.get(&addr).is_some() {
                    L3_LATENCY
                } else {
                    DRAM_LATENCY
                };

                self.pending_fetches.push(Pending {
                    tag,
                    current: 0,
                    end: latency,
                });

                false
            }
        }
    }

    pub fn finish_access(&mut self, tag: Tag, addr: Addr) {
        let pos = self
            .pending_fetches
            .iter()
            .position(|p| p.tag == tag)
            .unwrap();
        self.pending_fetches.swap_remove(pos);

        // Promote address to L1 cache
        let addr = addr.to_cache_line();
        if let Some((evicted, _)) = self.l1.insert(addr, WithLruTimestamp::new(())) {
            if let Some((evicted, _)) = self.l2.insert(evicted, WithLruTimestamp::new(())) {
                self.l3.insert(evicted, WithLruTimestamp::new(()));
            }
        }
    }

    pub fn tick(&mut self) {
        for p in &mut self.pending_fetches {
            p.current += 1;
        }
    }
}

impl Clone for MemoryHierarchy {
    fn clone(&self) -> Self {
        Self {
            main: self.main.clone(),
            l1: AssociativeCache::default(),
            l2: AssociativeCache::default(),
            l3: AssociativeCache::default(),
            pending_fetches: Vec::default(),
        }
    }
}

impl MainMemory {
    pub fn new() -> Self {
        Self {
            mem: vec![0; DRAM_CAPACITY_BYTES],
        }
    }

    pub fn readb(&self, addr: Addr) -> u32 {
        self.mem[addr.0 as usize] as u32
    }

    pub fn readh(&self, addr: Addr) -> u32 {
        let a = addr.0 as usize;
        assert!(a % 2 == 0);

        u16::from_le_bytes([self.mem[a], self.mem[a + 1]]) as u32
    }

    pub fn readw(&self, addr: Addr) -> u32 {
        let a = addr.0 as usize;
        assert!(a % 4 == 0);

        u32::from_le_bytes([
            self.mem[a],
            self.mem[a + 1],
            self.mem[a + 2],
            self.mem[a + 3],
        ])
    }

    pub fn writeb(&mut self, addr: Addr, val: u32) {
        self.mem[addr.0 as usize] = val.to_le_bytes()[0];
    }

    pub fn writeh(&mut self, addr: Addr, val: u32) {
        let a = addr.0 as usize;
        assert!(a % 2 == 0);

        self.mem[a..a + 2].copy_from_slice(&val.to_le_bytes())
    }

    pub fn writew(&mut self, addr: Addr, val: u32) {
        let a = addr.0 as usize;
        assert!(a % 4 == 0);

        self.mem[a..a + 4].copy_from_slice(&val.to_le_bytes())
    }
}

#[derive(Debug)]
struct CacheCapacity<const BYTES: usize>;

impl<const BYTES: usize> Capacity for CacheCapacity<{ BYTES }> {
    const CAPACITY: usize = BYTES / 64;
}

type CacheLevel<const BYTES: usize> = AssociativeCache<
    Addr,
    WithLruTimestamp<()>,
    CacheCapacity<BYTES>,
    HashEightWay,
    LruReplacement,
>;
type L1Cache = CacheLevel<L1_CAPACITY_BYTES>;
type L2Cache = CacheLevel<L2_CAPACITY_BYTES>;
type L3Cache = CacheLevel<L3_CAPACITY_BYTES>;
