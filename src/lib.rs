//! Execute a task in a new thread and await the result asynchronously.
//!
//! ```rust
//! use std::{thread, time};
//!
//! #[tokio::main]
//! async fn main() {
//!     let output = thread_async::run(|| {
//!         thread::sleep(time::Duration::from_millis(250));
//!         42
//!     })
//!     .await;
//!
//!     assert_eq!(output, 42);
//! }
//! ```

use std::{future, future::Future, io, sync, task, thread};

/// Executes a blocking task in a dedicated thread, returning a [`Future`] to await the result.
///
/// Each call to this function spawns a new thread that runs the blocking function and wakes the
/// current task after execution. This function is small and works with any async executor, though
/// it is suboptimal in terms of performance compared to equivalent functions provided by async
/// executor libraries.
///
/// # Panics
///
/// Panics if it fails to spawn a thread.
pub fn run<F, T>(blocking_fn: F) -> impl Future<Output = T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    run_with_builder(thread::Builder::new(), blocking_fn)
        .expect("failed to spawn thread")
        .0
}

/// Executes a blocking task in a dedicated thread, returning a [`Future`] to await the result.
///
/// For more information, see [`run()`]. Unlike `run()`, this function:
///
/// - Receives a [`thread::Builder`] to configure the thread.
/// - Returns an [`io::Result`] to report any failure in creating the thread.
/// - Returns a [`thread::JoinHandle`] to join the thread synchronously, in addition to the
///   [`Future`] to await the result.
///
/// # Errors
///
/// Returns `Err` if it fails to spawn a thread.
pub fn run_with_builder<F, T>(
    builder: thread::Builder,
    blocking_fn: F,
) -> io::Result<(impl Future<Output = T>, thread::JoinHandle<()>)>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let state: (Option<T>, Option<task::Waker>) = (None, None);
    let state_in_future = sync::Arc::new(sync::Mutex::new(state));
    let state_in_thread = state_in_future.clone();

    Ok((
        future::poll_fn(move |cx| {
            let mut state = state_in_future.lock().unwrap();
            match state.0.take() {
                Some(output) => task::Poll::Ready(output),
                None => {
                    match state.1.as_mut() {
                        Some(waker) => waker.clone_from(cx.waker()),
                        None => state.1 = Some(cx.waker().clone()),
                    }
                    task::Poll::Pending
                }
            }
        }),
        builder.spawn(move || {
            let output = blocking_fn();
            let mut state = state_in_thread.lock().unwrap();
            state.0 = Some(output);
            if let Some(waker) = state.1.take() {
                waker.wake();
            }
        })?,
    ))
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::{thread, time};

    const DUR: time::Duration = time::Duration::from_millis(250);
    const OUT: i32 = 42;

    fn blocking_task() -> i32 {
        thread::sleep(DUR);
        OUT
    }

    #[tokio::test]
    async fn single() {
        let start = time::Instant::now();
        let output = run(blocking_task).await;
        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
        assert_eq!(output, OUT);
    }

    #[tokio::test]
    async fn parallel() {
        let start = time::Instant::now();
        #[rustfmt::skip]
        tokio::join!(
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
            run(blocking_task), run(blocking_task), run(blocking_task), run(blocking_task),
        );
        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
    }

    #[tokio::test]
    async fn mix_with_tokio() {
        let start = time::Instant::now();
        #[rustfmt::skip]
        tokio::join!(
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
            run(blocking_task), tokio::time::sleep(DUR), run(blocking_task), tokio::time::sleep(DUR),
        );
        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
    }

    #[tokio::test]
    async fn delayed_await() {
        let start = time::Instant::now();
        let ft = run(blocking_task);
        thread::sleep(DUR * 125 / 100);
        let output = ft.await;
        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
        assert_eq!(output, OUT);
    }
}
