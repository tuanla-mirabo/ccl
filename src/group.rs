use super::stack::ConcurrentStack;
use rand::prelude::*;
use crossbeam_epoch::{self as epoch, Guard};

#[inline]
pub fn aquire_guard() -> Guard {
    epoch::pin()
}

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
        let guard = &aquire_guard();
        self.add_with_guard(element, guard);
    }

    #[inline]
    pub fn remove(&self) -> Option<T> {
        let guard = &aquire_guard();
        self.remove_with_guard(guard)
    }

    #[inline]
    pub fn add_with_guard(&self, element: T, guard: &Guard) {
        let segment_idx = rand::thread_rng().gen_range(0, self.segment_count);
        self.segments[segment_idx].push_with_guard(element, guard);
    }

    #[inline]
    pub fn remove_with_guard(&self, guard: &Guard) -> Option<T> {
        let segment_idx_initial = rand::thread_rng().gen_range(0, self.segment_count);
        let mut segment_idx = segment_idx_initial;

        loop {
            if let Some(elem) = self.segments[segment_idx].pop_with_guard(guard) {
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
            guard: aquire_guard(),
            group: &self,
        }
    }
}

pub struct GroupIter<'a, T> {
    guard: Guard,
    group: &'a Group<T>,
}

impl<'a, T> Iterator for GroupIter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.group.remove_with_guard(&self.guard)
    }
}

impl<'a, T> Drop for GroupIter<'a, T> {
    fn drop(&mut self) {
        self.guard.repin();
        self.guard.flush();
    }
}

impl<T> Default for Group<T> {
    fn default() -> Self {
        Self::new()
    }
}
