use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct OneShotChannel<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    set: AtomicBool,
    in_use: AtomicBool
}

impl<T> OneShotChannel<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            set: AtomicBool::new(false),
            in_use: AtomicBool::new(false)
        }
    }

    pub fn enqueue(&self, message: T) -> () {
        if self.in_use.swap(true, Acquire) {
            panic!("Message has already been queued.");
        }
        unsafe { (*self.value.get()).write(message); }
        self.set.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.set.load(Relaxed)
    }

    pub fn dequeue(&self) -> T {
        if !self.set.swap(false, Acquire) {
            panic!("No message is available");
        }
        let v = unsafe { (*self.value.get()).assume_init_read() };
        self.in_use.store(false, Release);
        v
    }
}

impl<T> Drop for OneShotChannel<T> {
    fn drop(&mut self) {
        if *self.set.get_mut() {
            unsafe {
                self.value.get_mut().assume_init_drop();
            }
        }
    }
}

unsafe impl<T> Sync for OneShotChannel<T> where T: Send {}

#[cfg(test)]
mod test {
    use crate::channels::one_shot::OneShotChannel;

    #[test]
    fn simple_roundtrip() {
        let channel = OneShotChannel::<i32>::new();
        let current_thread = std::thread::current();

        std::thread::scope(|s| {
            s.spawn(|| {
                channel.enqueue(55);
                current_thread.unpark();
            });
        });

        while !channel.is_ready() {
            std::thread::park();
        }

        let v = channel.dequeue();
        assert_eq!(55, v);
    }
}