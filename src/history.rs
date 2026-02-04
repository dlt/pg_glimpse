use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut data = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            data.push_back(T::default());
        }
        Self { data, capacity }
    }

    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn as_vec(&self) -> Vec<T> {
        self.data.iter().copied().collect()
    }

    pub fn last(&self) -> Option<T> {
        self.data.back().copied()
    }

    #[allow(dead_code)]
    pub fn peak(&self) -> T
    where
        T: Ord,
    {
        self.data.iter().copied().max().unwrap_or_default()
    }
}
