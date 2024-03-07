use crate::spin_lock::{SpinLock};
use std::collections::VecDeque;

pub struct Channel<T> {
    data: SpinLock<VecDeque<T>>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            data: SpinLock::new(VecDeque::new()),
        }
    }

    pub fn enqueue(&self, data: T) -> () {
        let mut inner = self.data.lock();
        inner.push_back(data);
    }

    pub fn dequeue(&self) -> T {
        loop {
            let mut inner = self.data.lock();
            if inner.is_empty() {
                std::hint::spin_loop();
            } else {
                let front = inner.pop_front().unwrap();
                return front;
            }
        }
    }
}

unsafe impl<T> Sync for Channel<T> {}

#[cfg(test)]
mod test {
    use crate::channel::Channel;

    #[test]
    fn simple_channel_test() {
        let channel = Channel::<i32>::new();
        let mut sum = 0;
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for i in 0..6 {
                    let n = channel.dequeue();
                    sum += n;
                }
            });

            scope.spawn(|| {
                for i in 0..6 {
                    channel.enqueue(i);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            });
        });

        assert_eq!(15, sum);
    }
}
