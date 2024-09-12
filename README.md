# Crate thread_async

[![Crates.io](https://img.shields.io/crates/v/thread-async)](https://crates.io/crates/thread-async)
[![License](https://img.shields.io/crates/l/thread-async)](https://github.com/LiosK/thread-async-rs/blob/main/LICENSE)

Execute a task in a new thread and await the result asynchronously.

```rust
use std::{thread, time};

#[tokio::main]
async fn main() {
    let output = thread_async::run(|| {
        thread::sleep(time::Duration::from_millis(250));
        42
    })
    .await;

    assert_eq!(output, 42);
}
```
