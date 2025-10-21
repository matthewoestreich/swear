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
        })
        .block();
}
