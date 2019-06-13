//! Please see the struct level documentation.

use std::sync::atomic::Ordering;
use std::ptr;
use crossbeam_epoch::{self as epoch, Atomic, Owned, Guard, Pointer};
use std::mem;

#[inline]
pub fn aquire_guard() -> Guard {
    epoch::pin()
}

/// ConcurrentStack is a general purpose threadsafe and lockfree FILO/LIFO stack.
pub struct ConcurrentStack<T> {
    head: Atomic<Node<T>>,
}

struct Node<T> {
    data: T,
    next: Atomic<Node<T>>,
}

impl<T> ConcurrentStack<T> {
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    #[inline]
    pub fn push(&self, data: T) {
        let guard = &aquire_guard();
        self.push_with_guard(data, guard);
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        let guard = &aquire_guard();
        self.pop_with_guard(guard)
    }

    #[inline]
    pub fn push_with_guard(&self, data: T, guard: &Guard) {
        let mut node = Owned::new(Node {
            data,
            next: Atomic::null(),
        });

        loop {
            let head = self.head.load(Ordering::SeqCst, guard);

            node.next.store(head, Ordering::SeqCst);

            match self.head.compare_and_set(head, node, Ordering::SeqCst, guard) {
                Ok(_) => return,
                Err(err) => node = err.new,
            }
        }
    }

    #[inline]
    pub fn pop_with_guard(&self, guard: &Guard) -> Option<T> {
        loop {
            let head_ptr = self.head.load(Ordering::SeqCst, guard);

            match unsafe { head_ptr.as_ref() } {
                Some(head) => unsafe {
                    let next = head.next.load(Ordering::SeqCst, guard);

                    if let Ok(head_ptr) = self.head.compare_and_set(head_ptr, next, Ordering::SeqCst, guard) {
                        guard.defer_unchecked(move || {
                            mem::drop(Box::from_raw(head_ptr.into_usize() as *mut Node<T>));
                        });

                        return Some(ptr::read(&(*head).data));
                    }
                }
                None => return None,
            }
        }
    }
}

impl<T> Default for ConcurrentStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[test]
    fn insert_then_pop_assert_1024_st() {
        let stack = ConcurrentStack::new();

        for _ in 0..1024_i32 {
            stack.push(9);
        }

        for _ in 0..1024_i32 {
            assert_eq!(9, stack.pop().unwrap());
        }
    }

    #[test]
    fn insert_then_pop_assert_rayon() {
        let stack = ConcurrentStack::new();

        let iter_c: i32 = 1024 * 1024;

        (0..iter_c).into_par_iter().for_each(|_| {
            stack.push(9);
        });

        (0..iter_c).into_par_iter().for_each(|_| {
            assert_eq!(9, stack.pop().unwrap());
        });
    }
}
