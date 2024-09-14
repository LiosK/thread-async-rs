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
//!
//! [`run()`] and its underlying primitive, [`run_with_builder()`], execute the
//! specified function in a separate thread and return a [`Future`] to `.await` the
//! result. Each call to `run()` or `run_with_builder()` spawns a new thread that
//! executes the specified function and wakes the current task upon completion. The
//! specified function is triggered at the time of the call to `run() ` or
//! `run_with_builder()`, not at the time of `.await`.
//!
//! This small crate is portable and works with any async executor, though it is
//! suboptimal in performance as it creates a new thread for each task. Equivalent
//! functions provided by async executors are usually recommended, unless a
//! lightweight, executor-agnostic solution is specifically desired.

use std::{future, future::Future, io, sync, task, thread};

/// Executes a task in a new thread, returning a [`Future`] to `.await` the result.
///
/// See [the crate-level documentation](crate) for details.
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

/// Executes a task in a new thread configured by a [`Builder`](thread::Builder),
/// returning a [`Result`](io::Result) wrapping a [`Future`] to `.await` the result and a
/// [`JoinHandle`](thread::JoinHandle) to join on the thread.
///
/// See [the crate-level documentation](crate) for details. Unlike [`run()`], this function:
///
/// - Receives a [`thread::Builder`] to configure the thread.
/// - Returns an [`io::Result`] to report any failure in creating the thread.
/// - Returns a [`thread::JoinHandle`] to join on the thread synchronously, in addition to the
///   `Future` to `.await` the result.
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
    let state_in_thread = sync::Arc::clone(&state_in_future);

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
    use super::{run, run_with_builder};
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

    #[tokio::test]
    async fn builder() {
        let name = "test run_with_builder()";
        let builder = thread::Builder::new().name(name.into());
        let start = time::Instant::now();
        let (ft, jh) = run_with_builder(builder, blocking_task).unwrap();
        assert_eq!(jh.thread().name(), Some(name));
        assert!(!jh.is_finished());
        let output = ft.await;
        assert!(jh.is_finished());
        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
        assert_eq!(output, OUT);
    }

    #[test]
    fn sync_wait() {
        use std::{future::Future as _, pin, sync, task};

        struct MockWaker(sync::Mutex<u8>);
        impl task::Wake for MockWaker {
            fn wake(self: sync::Arc<Self>) {
                *self.0.lock().unwrap() += 1;
            }
        }
        let waker_inner = sync::Arc::new(MockWaker(Default::default()));
        let waker = sync::Arc::clone(&waker_inner).into();
        let mut context = task::Context::from_waker(&waker);

        let builder = thread::Builder::new();
        let start = time::Instant::now();
        let (ft, jh) = run_with_builder(builder, blocking_task).unwrap();
        let mut ft = pin::pin!(ft);

        let poll_result = ft.as_mut().poll(&mut context);
        assert!(!jh.is_finished());
        assert_eq!(poll_result, task::Poll::Pending);
        let poll_result = ft.as_mut().poll(&mut context);
        assert!(!jh.is_finished());
        assert_eq!(poll_result, task::Poll::Pending);

        jh.join().unwrap();
        let poll_result = ft.as_mut().poll(&mut context);
        assert_eq!(poll_result, task::Poll::Ready(OUT));

        let elapsed = time::Instant::now().duration_since(start);
        assert!(DUR <= elapsed && elapsed < DUR * 2);
        assert_eq!(*waker_inner.0.lock().unwrap(), 1);
    }
}
