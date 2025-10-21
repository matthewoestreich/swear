use rand::Rng;
use std::{error, fmt};
use swear::Swear;

#[derive(Debug, Clone)]
pub enum MyError {
    New(String),
}

impl error::Error for MyError {}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MyError::*;
        match self {
            New(s) => write!(f, "{s}"),
        }
    }
}

fn foo() -> Swear<i32, MyError> {
    Swear::new(|resolve, reject| {
        let mut rng = rand::rng();
        if rng.random_bool(0.5) {
            resolve(rng.random_range(0..100));
        } else {
            reject(MyError::New("Random failure".to_owned()));
        }
    })
}

fn main() {
    foo()
        .then(|v| {
            println!("SUCCESS : {v}");
        })
        .catch(|err| {
            println!("ERROR : {err:?}");
        });

    std::thread::sleep(std::time::Duration::from_secs(1));

    Swear::new(|resolve, reject| {
        let now = std::time::SystemTime::now();
        let ns = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        if ns % 2 == 0 {
            resolve(ns);
        } else {
            reject("Some Error");
        }
    })
    .then(|n| println!("I am an i32 {n}"))
    .catch(|e| println!("I am SomeError {e:?}"))
    .block();
}
