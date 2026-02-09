use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_buffer() {
        let buf: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(buf.as_vec(), Vec::<i32>::new());
        assert_eq!(buf.last(), None);
    }

    #[test]
    fn push_adds_elements() {
        let mut buf = RingBuffer::new(5);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        assert_eq!(buf.as_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn push_evicts_oldest_when_at_capacity() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);
        assert_eq!(buf.as_vec(), vec![2, 3, 4]);
        buf.push(5);
        assert_eq!(buf.as_vec(), vec![3, 4, 5]);
    }

    #[test]
    fn last_returns_most_recent() {
        let mut buf = RingBuffer::new(5);
        assert_eq!(buf.last(), None);
        buf.push(10);
        assert_eq!(buf.last(), Some(10));
        buf.push(20);
        assert_eq!(buf.last(), Some(20));
    }

    #[test]
    fn peak_returns_max_element() {
        let mut buf = RingBuffer::new(5);
        buf.push(3);
        buf.push(1);
        buf.push(4);
        buf.push(1);
        buf.push(5);
        assert_eq!(buf.peak(), 5);
    }

    #[test]
    fn peak_returns_default_when_empty() {
        let buf: RingBuffer<i32> = RingBuffer::new(5);
        assert_eq!(buf.peak(), 0);
    }

    #[test]
    fn capacity_one() {
        let mut buf = RingBuffer::new(1);
        buf.push(1);
        assert_eq!(buf.as_vec(), vec![1]);
        buf.push(2);
        assert_eq!(buf.as_vec(), vec![2]);
        assert_eq!(buf.last(), Some(2));
    }

    #[test]
    fn maintains_order_after_wrap() {
        let mut buf = RingBuffer::new(3);
        for i in 1..=10 {
            buf.push(i);
        }
        assert_eq!(buf.as_vec(), vec![8, 9, 10]);
    }
}
