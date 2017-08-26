use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

type Job<T> = Message<T>;
pub enum Message<T> {
    Work(Box<Fn() -> T + Send + 'static>),
    Terminate,
}


pub struct ThreadPool <T>{
    sender: mpsc::Sender<Job<T>>,
    pub result: mpsc::Receiver<T>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl<T> ThreadPool<T> where T: Send + 'static{
    pub fn new(n: usize, s: u64) -> Self {
        // send works to worker
        let (tx, rx) = mpsc::channel();
        // send result from worker
        let (tx1, rx1) = mpsc::channel();
        let rx : Arc<Mutex<mpsc::Receiver<Job<T>>>> = Arc::new(Mutex::new(rx));
        let v = (0 .. n).map(| _ | {
            let rx = rx.clone();
            let tx1 : mpsc::Sender<T> = tx1.clone();
            thread::spawn(move || 
                loop {
                    if let Ok(f) = rx.lock() {
                        if let Ok(f) = f.recv() {
                            match f {
                                Message::Work(f) => {
                                    let r : T = f();
                                    tx1.send(r);
                                },
                                Message::Terminate => break,
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(s));
                }
            )
        }).collect::<Vec<_>>();


        ThreadPool {
            sender: tx,
            result: rx1,
            threads: v,
        }
    }

    pub fn add(&self, f: Job<T>) {
        self.sender.send(f);
    }
}

impl<T> Drop for ThreadPool<T> {
    fn drop(&mut self) {
        for _ in &self.threads {
            self.sender.send(Message::Terminate);
        }
        while let Some(t) = self.threads.pop() {
            t.join().unwrap();
        }
    }
}

#[test]
fn threads() {
    let tp = ThreadPool::new(10, 100);
    for i in 0 .. 100 {
        tp.add(
            Message::Work(
            Box::new(move || { println!("Thread: {}", i); i*100 })));
   }
    let mut c = 0;
   loop {
        if let Ok(s) = tp.result.recv() {
            println!("Result: {}", s);
            c += 1;
        } else {
            thread::sleep(Duration::from_millis(10));
        }
        if c == 100 {
            break;
        }
    }
}
