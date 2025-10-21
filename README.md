# Swear

Promise-like behavior.

At a high level:

```rust
// You can also do something like
Swear::new(|resolve, reject| {
  // Do something...
  if /* it was successful */ {
    resolve(the_result);
  }
  if /* there was an error */ {
    reject(the_error);
  }
})
.then(|value| { /* ... */ })
.catch(|error| { /* ... */ })
.block();
```

Return a `Swear` from a `fn`

```rust
// Say you have a fn that returns a Swear
fn foo() -> Swear<i32, SomeError> { /* ... */ }

// You can do
foo()
  .then(|n| println!("I am an i32 {n}"))
  .catch(|e| println!("I am SomeError {e:?}"));

// Keep in mind, this does not block the main thread...

// If you want to block the main thread, you can call `block()`:
foo()
  .then(|n| println!("I am an i32 {n}"))
  .catch(|e| println!("I am SomeError {e:?}"))
  .block();
```
