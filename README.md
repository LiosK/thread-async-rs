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

`run()` and its underlying primitive, `run_with_builder()`, execute the
specified function in a separate thread and return a `Future` to `.await` the
result. Each call to `run()` or `run_with_builder()` spawns a new thread that
executes the specified function and wakes the current task upon completion. The
specified function is triggered at the time of the call to `run() ` or
`run_with_builder()`, not at the time of `.await`.

This small crate is portable and works with any async executor, though it is
suboptimal in performance as it creates a new thread for each task. Equivalent
functions provided by async executors are usually recommended, unless a
lightweight, executor-agnostic solution is specifically desired.
