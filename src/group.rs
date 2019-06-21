use super::stack::ConcurrentStack;
use ccl_crossbeam_epoch::{self as epoch, Guard};
use rand::prelude::*;

/// Aquire a guard. These are needed when accessing a group. Since aquiring a guard has a significant cost,
/// you may wish to aquire a guard once and pass it around when doing bulk operations.
/// For most use cases you will not need this.
///
/// Please note that no memory consumed by objects removed after the guard was aquired can be reclaimed
/// until the guard has been dropped.
#[inline]
pub fn aquire_guard() -> Guard {
    epoch::pin()
}

/// A unordered container for a group of elements. This container should be used when you want to group a lot of elements
/// from a multithreaded context so that they then can be used in a singlethreaded context. This structure makes no guarantees
/// about how elements are stored and in which order.
pub struct Group<T> {
    segment_count: usize,
    segments: Box<[ConcurrentStack<T>]>,
}

impl<T> Group<T> {
    /// Create a new, empty group.
    pub fn new() -> Self {
        let segment_count = num_cpus::get();

        Self {
            segment_count,
            segments: (0..segment_count)
                .map(|_| ConcurrentStack::new())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Add an element to the group.
    #[inline]
    pub fn add(&self, element: T) {
        let guard = &aquire_guard();
        self.add_with_guard(element, guard);
    }

    /// Remove an element from the group, returning it.
    #[inline]
    pub fn remove(&self) -> Option<T> {
        let guard = &aquire_guard();
        self.remove_with_guard(guard)
    }

    /// Add an element with an existing guard.
    #[inline]
    pub fn add_with_guard(&self, element: T, guard: &Guard) {
        let segment_idx = rand::thread_rng().gen_range(0, self.segment_count);
        self.segments[segment_idx].push_with_guard(element, guard);
    }

    /// Remove an element with an existing guard.
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

    /// Create an iterator over all elements in the group, removing them.
    #[inline]
    pub fn remove_iter(&self) -> GroupIter<T> {
        GroupIter {
            guard: aquire_guard(),
            group: &self,
        }
    }
}

/// An iterator over a group.
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

impl<T> Default for Group<T> {
    fn default() -> Self {
        Self::new()
    }
}
