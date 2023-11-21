use dialoguer::theme::Theme;
use log::{debug, trace, warn};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub struct Threadpool {
    workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;
impl Threadpool {
    pub fn new(size: usize) -> Self {
        trace!("Initialising threadpool");
        assert!(size > 0, "size of thread pool must be greater than 0");

        let mut workers = Vec::with_capacity(size);
        let (sender, reciever) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(reciever));

        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&receiver)));
            trace!("Initialised thread {} of {size}", i + 1)
        }

        debug!("Threadpool initialised");

        Threadpool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn exec<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for Threadpool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            drop(self.sender.take());
            debug!("Shutting down worker {}", worker.id);

            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }

            trace!("Shut down worker {}", worker.id);
        }
    }
}

struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let handle = thread::Builder::new()
            .name(format!("Worker {id}"))
            .spawn(move || loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        trace!("Worker {id} got a job; executing.");

                        job();
                    }
                    Err(_) => {
                        debug!("Worker {id} disconnected; shutting down.");
                        break;
                    }
                }
            })
            .unwrap();

        Self {
            id,
            handle: Some(handle),
        }
    }
}
