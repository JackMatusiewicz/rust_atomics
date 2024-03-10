use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool
}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>
}

unsafe impl<T> Sync for Channel<T> where T : Send {}

impl<T> Channel<T> {
    fn new_channel() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false)
        }
    }

    pub fn new(channel: &mut Channel<T>) -> (Sender<T>, Receiver<T>) {
        *channel = Self::new_channel();
        (Sender {channel}, Receiver {channel})
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe {
                self.message.get_mut().assume_init_drop();
            }
        }
    }
}

impl<T> Sender<'_, T> {
    pub fn send(self, message: T) -> () {
        unsafe { (*self.channel.message.get()).write(message); }
        self.channel.ready.store(true, Release);
    }
}

impl<T> Receiver<'_, T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    pub fn receive(self) -> T {
        if !self.channel.ready.swap(false, Acquire) {
            panic!("Message has not been set yet.");
        }
        unsafe {
            (*self.channel.message.get()).assume_init_read()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::channels::one_shot_borrow::Channel;

    #[test]
    fn simple_roundtrip() {
        let mut chan = Channel::new_channel();
        let (ts, tr) = Channel::<i32>::new(&mut chan);
        let current_thread = std::thread::current();

        std::thread::scope(|s| {
            s.spawn(move || {
                ts.send(55);
                current_thread.unpark();
            });
        });

        while !tr.is_ready() {
            std::thread::park();
        }

        let v = tr.receive();

        assert_eq!(55, v);
    }
}