use std::{
    sync::{Arc, Condvar, Mutex},
    thread,
};

pub enum SwearState<T, E> {
    Pending,
    Settled(T),
    Rejected(E),
}

pub type Resolve<T> = Box<dyn FnOnce(T) + Send>;
pub type Reject<E> = Box<dyn FnOnce(E) + Send>;
pub type ThenQueue<T> = Arc<Mutex<Vec<Resolve<T>>>>;
pub type CatchQueue<E> = Arc<Mutex<Vec<Resolve<E>>>>;

#[derive(Clone)]
pub struct Swear<T, E> {
    status: Arc<Mutex<SwearState<T, E>>>,
    then_queue: ThenQueue<T>,
    catch_queue: CatchQueue<E>,
}

impl<T: Send + Clone + 'static, E: Send + Clone + 'static> Swear<T, E> {
    pub fn new(callback: impl FnOnce(Resolve<T>, Reject<E>) + Send + 'static) -> Self {
        let status = Arc::new(Mutex::new(SwearState::Pending));
        let then_queue: ThenQueue<T> = Arc::new(Mutex::new(Vec::new()));
        let catch_queue: CatchQueue<E> = Arc::new(Mutex::new(Vec::new()));

        let status_resolve = Arc::clone(&status);
        let then_queue_resolve = Arc::clone(&then_queue);
        let resolve: Resolve<T> = Box::new(move |val: T| {
            let mut st = status_resolve.lock().unwrap();
            if matches!(*st, SwearState::Pending) {
                *st = SwearState::Settled(val.clone());
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
            if matches!(*st, SwearState::Pending) {
                *st = SwearState::Rejected(err.clone());
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

    pub fn then(&self, cb: impl FnOnce(T) + Send + 'static) -> Self {
        match &*self.status.lock().unwrap() {
            SwearState::Settled(val) => {
                let val_clone = val.clone();
                thread::spawn(move || cb(val_clone));
            }
            SwearState::Pending => {
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

    pub fn catch(&self, cb: impl FnOnce(E) + Send + 'static) -> Self {
        match &*self.status.lock().unwrap() {
            SwearState::Rejected(err) => {
                let err_clone = err.clone();
                thread::spawn(move || cb(err_clone));
            }
            SwearState::Pending => {
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
