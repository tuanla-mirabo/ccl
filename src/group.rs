use super::stack::ConcurrentStack;
use rand::prelude::*;

pub struct Group<T> {
    segment_count: usize,
    segments: Box<[ConcurrentStack<T>]>,
}

impl<T> Group<T> {
    pub fn new() -> Self {
        let segment_count = num_cpus::get();

        Self {
            segment_count,
            segments: (0..segment_count).map(|_| ConcurrentStack::new()).collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    #[inline]
    pub fn add(&self, element: T) {
        let segment_idx = rand::thread_rng().gen_range(0, self.segment_count);
        self.segments[segment_idx].push(element);
    }

    #[inline]
    pub fn remove(&self) -> Option<T> {
        let segment_idx_initial = rand::thread_rng().gen_range(0, self.segment_count);
        let mut segment_idx = segment_idx_initial;

        loop {
            if let Some(elem) = self.segments[segment_idx].pop() {
                return Some(elem);
            } else {
                segment_idx = (segment_idx + 1) % self.segment_count;
            }

            if segment_idx == segment_idx_initial {
                return None;
            }
        }
    }

    #[inline]
    pub fn remove_iter(&self) -> GroupIter<T> {
        GroupIter {
            group: &self,
        }
    }
}

pub struct GroupIter<'a, T> {
    group: &'a Group<T>,
}

impl<'a, T> Iterator for GroupIter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.group.remove()
    }
}

impl<T> Default for Group<T> {
    fn default() -> Self {
        Self::new()
    }
}
