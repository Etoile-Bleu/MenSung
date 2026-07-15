//! Shared retry-with-backoff helper for this crate's network fetchers. A
//! flaky connection (Wi-Fi drop, a mobile hotspot toggled off mid-run, a
//! brief hiccup on NLM's or PubChem's end) shows up here as a single
//! transport error in the middle of an otherwise-healthy hour-long
//! pipeline run; retrying a few times with a growing pause turns that
//! into a few seconds' delay instead of losing the whole run and having
//! to restart from scratch.

use std::thread;
use std::time::Duration;

const MAX_ATTEMPTS: u32 = 4;
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);

/// Calls `f`, retrying up to `MAX_ATTEMPTS` times on failure with
/// exponentially growing backoff (2s, 4s, 8s between attempts). Prints a
/// line to stderr for each retry so a long-running, backgrounded pipeline
/// shows why it paused instead of looking hung. Returns the last error if
/// every attempt fails.
pub fn with_retry<T, E: std::fmt::Display>(
    what: &str,
    f: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    with_retry_from(what, INITIAL_BACKOFF, f)
}

/// The actual retry loop, taking the initial backoff as a parameter so
/// tests can run it with a near-zero delay instead of the real multi-
/// second backoff `with_retry` uses in production.
fn with_retry_from<T, E: std::fmt::Display>(
    what: &str,
    initial_backoff: Duration,
    mut f: impl FnMut() -> Result<T, E>,
) -> Result<T, E> {
    let mut backoff = initial_backoff;
    let mut last_err = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match f() {
            Ok(value) => return Ok(value),
            Err(err) => {
                if attempt < MAX_ATTEMPTS {
                    eprintln!(
                        "    retry {attempt}/{MAX_ATTEMPTS} for {what} after error: {err} \
                         (waiting {}s)",
                        backoff.as_secs_f64()
                    );
                    thread::sleep(backoff);
                    backoff *= 2;
                }
                last_err = Some(err);
            }
        }
    }

    Err(last_err.expect(
        "the loop above runs MAX_ATTEMPTS >= 1 times, recording an error on every non-Ok path",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const NEGLIGIBLE_TEST_BACKOFF: Duration = Duration::from_millis(1);

    #[test]
    fn returns_the_first_ok_without_retrying() {
        let calls = Cell::new(0);
        let result = with_retry_from("test", NEGLIGIBLE_TEST_BACKOFF, || {
            calls.set(calls.get() + 1);
            Ok::<_, &str>("value")
        });
        assert_eq!(result, Ok("value"));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn retries_after_a_transient_failure_and_then_succeeds() {
        let calls = Cell::new(0);
        let result = with_retry_from("test", NEGLIGIBLE_TEST_BACKOFF, || {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err("transient")
            } else {
                Ok("value")
            }
        });
        assert_eq!(result, Ok("value"));
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn gives_up_after_max_attempts_and_returns_the_last_error() {
        let calls = Cell::new(0);
        let result = with_retry_from("test", NEGLIGIBLE_TEST_BACKOFF, || {
            calls.set(calls.get() + 1);
            Err::<&str, _>(format!("failure #{}", calls.get()))
        });
        assert_eq!(calls.get(), MAX_ATTEMPTS);
        assert_eq!(result, Err(format!("failure #{MAX_ATTEMPTS}")));
    }
}
