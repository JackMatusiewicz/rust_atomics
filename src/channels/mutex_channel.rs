use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

pub struct MutexChannel<T> {
    data: Mutex<VecDeque<T>>,
    wait: Condvar
}

impl<T> MutexChannel<T> {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
            wait: Condvar::new()
        }
    }

    pub fn enqueue(&self, message: T) -> () {
        self.data.lock().unwrap().push_back(message);
        self.wait.notify_one();
    }

    pub fn dequeue(&self) -> T {
        let mut d = self.data.lock().unwrap();
        loop {
            if let Some(v) = d.pop_front() {
                return v;
            }
            d = self.wait.wait(d).unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::channels::mutex_channel::MutexChannel;

    #[test]
    fn simple_channel_test() {
        let channel = MutexChannel::<i32>::new();
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