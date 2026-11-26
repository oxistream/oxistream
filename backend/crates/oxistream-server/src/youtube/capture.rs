//! Captures the raw YouTube responses behind a RustyPipe call, so fields that RustyPipe fails
//! to parse can be read from them without a second request.

use std::sync::{Arc, Mutex, PoisonError};

use rustypipe::report::{Report, Reporter};

tokio::task_local! {
    static BODIES: Arc<Mutex<Vec<String>>>;
}

/// Receives a report for every request made with `RustyPipeQuery::report`; RustyPipe calls it
/// synchronously from the task that awaits the query.
pub(super) struct CaptureReporter;

impl Reporter for CaptureReporter {
    fn report(&self, report: &Report) {
        let _ = BODIES.try_with(|bodies| {
            bodies
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(report.http_request.resp_body.clone());
        });
    }
}

/// Runs `fut` and returns its output along with the response bodies it received.
pub(super) async fn capture<F: Future>(fut: F) -> (F::Output, Vec<String>) {
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let output = BODIES.scope(Arc::clone(&bodies), fut).await;
    let bodies = std::mem::take(&mut *bodies.lock().unwrap_or_else(PoisonError::into_inner));
    (output, bodies)
}
