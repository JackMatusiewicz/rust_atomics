use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};

// This structure needs to exist for the safe interface where a lock returns an object that
// allows us to use the data inside for a window of time. It implements the Drop trait to unlock
// the spin lock the moment that the guard value is dropped.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

// Required so that we can automatically unlock the underlying spinlock when the value goes out
// of scope.
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.is_locked.store(false, Release);
    }
}

// Required so that we can treat a spinLockGuard<_,T> as a reference to a T value.
impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { & *self.lock.data.get() }
    }
}

// Required so that we can treat a mutable spinLockGuard<_,T> as a mutable reference to a T value.
impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub struct SpinLock<T> {
    is_locked: AtomicBool,
    // We add this so that the interface grants access to the data when the lock is taken,
    // rather than relying on users to correctly use the lock with locked data.
    data: UnsafeCell<T>
}

impl<T> SpinLock<T> {
    pub fn new(v: T) -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            data: UnsafeCell::new(v)
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        while self.is_locked.swap(true, Acquire) {
            std::hint::spin_loop();
        }
        return SpinLockGuard { lock: self };
    }
}

unsafe impl<T> Send for SpinLock<T> {
}

unsafe impl<T> Sync for SpinLock<T> {
}

#[cfg(test)]
mod test {
    use std::ops::Deref;
    use crate::spin_lock::SpinLock;

    #[test]
    fn simple_test() {
        let sl = SpinLock::new(Vec::<i32>::new());
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let mut g = sl.lock();
                g.push(1);
                g.push(2);
                g.push(3);
            });
            scope.spawn(|| {
                let mut g = sl.lock();
                g.push(4);
                g.push(5);
            });
        });

        let g = sl.lock();
        let vec = g.deref();
        let sum = vec.iter().sum();
        assert_eq!(15, sum);
    }
}