use std::{
    sync::{Arc, Condvar, Mutex},
    thread,
};

type ThenQueue<T> = Arc<Mutex<Vec<Resolve<T>>>>;
type CatchQueue<E> = Arc<Mutex<Vec<Reject<E>>>>;

pub enum SwearStatus<T, E> {
    Pending,
    Settled(T),
    Rejected(E),
}

pub trait ThreadSafeClone: Send + Clone + 'static {}
impl<T: Send + Clone + 'static> ThreadSafeClone for T {}

pub type Resolve<T> = Box<dyn FnOnce(T) + Send>;
pub type Reject<E> = Box<dyn FnOnce(E) + Send>;

#[derive(Clone)]
pub struct Swear<T, E> {
    status: Arc<Mutex<SwearStatus<T, E>>>,
    then_queue: ThenQueue<T>,
    catch_queue: CatchQueue<E>,
}

impl<T, E> Swear<T, E>
where
    T: ThreadSafeClone,
    E: ThreadSafeClone,
{
    pub fn new<F>(callback: F) -> Self
    where
        F: FnOnce(Resolve<T>, Reject<E>) + Send + 'static,
    {
        let status = Arc::new(Mutex::new(SwearStatus::Pending));
        let then_queue: ThenQueue<T> = Arc::new(Mutex::new(Vec::new()));
        let catch_queue: CatchQueue<E> = Arc::new(Mutex::new(Vec::new()));

        let status_resolve = Arc::clone(&status);
        let then_queue_resolve = Arc::clone(&then_queue);
        let resolve: Resolve<T> = Box::new(move |val: T| {
            let mut st = status_resolve.lock().unwrap();
            if matches!(*st, SwearStatus::Pending) {
                *st = SwearStatus::Settled(val.clone());
                let mut queue = then_queue_resolve.lock().unwrap();
                for cb in queue.drain(..) {
                    let val_clone = val.clone();
                    thread::spawn(move || cb(val_clone));
                }
            }
        });

        let status_reject = Arc::clone(&status);
        let catch_queue_reject = Arc::clone(&catch_queue);
        let reject: Reject<E> = Box::new(move |err: E| {
            let mut st = status_reject.lock().unwrap();
            if matches!(*st, SwearStatus::Pending) {
                *st = SwearStatus::Rejected(err.clone());
                let mut queue = catch_queue_reject.lock().unwrap();
                for cb in queue.drain(..) {
                    let err_clone = err.clone();
                    thread::spawn(move || cb(err_clone));
                }
            }
        });

        thread::spawn(move || {
            callback(resolve, reject);
        });

        Swear {
            status,
            then_queue,
            catch_queue,
        }
    }

    pub fn then<F>(&self, cb: F) -> Self
    where
        F: FnOnce(T) + Send + 'static,
    {
        match &*self.status.lock().unwrap() {
            SwearStatus::Settled(val) => {
                let val_clone = val.clone();
                thread::spawn(move || cb(val_clone));
            }
            SwearStatus::Pending => {
                self.then_queue.lock().unwrap().push(Box::new(cb));
            }
            _ => {}
        }

        Swear {
            status: Arc::clone(&self.status),
            then_queue: Arc::clone(&self.then_queue),
            catch_queue: Arc::clone(&self.catch_queue),
        }
    }

    pub fn catch<F>(&self, cb: F) -> Self
    where
        F: FnOnce(E) + Send + 'static,
    {
        match &*self.status.lock().unwrap() {
            SwearStatus::Rejected(err) => {
                let err_clone = err.clone();
                thread::spawn(move || cb(err_clone));
            }
            SwearStatus::Pending => {
                self.catch_queue.lock().unwrap().push(Box::new(cb));
            }
            _ => {}
        }

        Swear {
            status: Arc::clone(&self.status),
            then_queue: Arc::clone(&self.then_queue),
            catch_queue: Arc::clone(&self.catch_queue),
        }
    }

    pub fn block(&self) {
        let pair = Arc::new((Mutex::new(None::<Result<T, E>>), Condvar::new()));
        let pair_clone = Arc::clone(&pair);

        self.then({
            let pair = Arc::clone(&pair_clone);
            move |v| {
                let (lock, cvar) = &*pair;
                let mut guard = lock.lock().unwrap();
                *guard = Some(Ok(v));
                cvar.notify_one();
            }
        })
        .catch({
            let pair = Arc::clone(&pair_clone);
            move |e| {
                let (lock, cvar) = &*pair;
                let mut guard = lock.lock().unwrap();
                *guard = Some(Err(e));
                cvar.notify_one();
            }
        });

        // wait until we get a value
        let (lock, cvar) = &*pair_clone;
        let mut guard = lock.lock().unwrap();
        while guard.is_none() {
            guard = cvar.wait(guard).unwrap();
        }
    }
}
