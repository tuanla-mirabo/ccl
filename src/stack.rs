use std::sync::atomic::Ordering;
use std::ptr;
use crossbeam_epoch::{self as epoch, Atomic, Owned, Guard, Pointer};
use std::mem;

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

    pub fn push(&self, data: T) {
        let guard = &epoch::pin();
        self.push_with_guard(data, guard);
    }

    pub fn pop(&self) -> Option<T> {
        let guard = &epoch::pin();
        self.pop_with_guard(guard)
    }

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
