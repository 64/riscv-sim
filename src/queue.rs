use std::collections::{vec_deque, VecDeque};

#[derive(Debug, Clone)]
pub struct Queue<T: Clone> {
    data: VecDeque<T>,
}

impl<T: Clone> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
        }
    }

    #[must_use]
    pub fn try_push(&mut self, item: T) -> Option<T> {
        if self.is_full() {
            return Some(item);
        }

        self.data.push_back(item);
        None
    }

    pub fn try_pop(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn front(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn is_full(&self) -> bool {
        self.data.capacity() == self.data.len()
    }

    pub fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> vec_deque::IterMut<'_, T> {
        self.data.iter_mut()
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.data.retain(f)
    }
}
