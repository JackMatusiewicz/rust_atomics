use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::thread::Thread;

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,

}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
    receiver_thread: Thread
}

pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
    unused: PhantomData<*const ()>
}

unsafe impl<T> Sync for Channel<T> where T : Send {}

impl<T> Channel<T> {
    fn new_channel() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false)
        }
    }

    pub fn new() -> (Sender<T>, Receiver<T>) {
        let channel_arc = Arc::new(Self::new_channel());

        (Sender {channel: channel_arc.clone(), receiver_thread: std::thread::current()},
         Receiver {channel: channel_arc.clone(), unused: PhantomData::default()})
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

impl<T> Sender<T> {
    pub fn send(self, message: T) -> () {
        unsafe { (*self.channel.message.get()).write(message); }
        self.channel.ready.store(true, Release);
        self.receiver_thread.unpark();
    }
}

impl<T> Receiver<T> {
    pub fn receive(self) -> T {
        while !self.channel.ready.swap(false, Acquire) {
            std::thread::park();
        }
        unsafe {
            (*self.channel.message.get()).assume_init_read()
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use crate::channels::one_shot_blocking::Channel;

    #[test]
    fn simple_roundtrip() {
        let (ts, tr) = Channel::<i32>::new();

        std::thread::scope(|s| {
            s.spawn(move || {
                std::thread::sleep(Duration::from_millis(2000));
                ts.send(55);
            });
        });

        let v = tr.receive();
        assert_eq!(55, v);
    }
}