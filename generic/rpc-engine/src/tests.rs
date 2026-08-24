use std::time::Instant;

use super::ProgressThrottle;

/// `Instant` cannot be built from nothing, so the tests start from a real one
/// and move forward from it. Nothing here reads the clock again.
fn origin() -> Instant {
    Instant::now()
}

#[test]
fn progress_throttle_sends_the_first_event() {
    let mut throttle = ProgressThrottle::new();

    assert!(throttle.should_send(origin(), false));
}

#[test]
fn progress_throttle_drops_events_inside_the_interval() {
    let start = origin();
    let mut throttle = ProgressThrottle::new();

    assert!(throttle.should_send(start, false));
    assert!(!throttle.should_send(start, false));
    assert!(!throttle.should_send(start + ProgressThrottle::MIN_INTERVAL / 2, false));
}

#[test]
fn progress_throttle_sends_again_once_the_interval_has_passed() {
    let start = origin();
    let mut throttle = ProgressThrottle::new();

    assert!(throttle.should_send(start, false));
    assert!(throttle.should_send(start + ProgressThrottle::MIN_INTERVAL, false));

    // The clock restarts from the event that was sent, not from the last one
    // offered, so a burst in between does not bring the next send forward.
    let later = start + ProgressThrottle::MIN_INTERVAL;
    assert!(!throttle.should_send(later + ProgressThrottle::MIN_INTERVAL / 2, false));
    assert!(throttle.should_send(later + ProgressThrottle::MIN_INTERVAL, false));
}

#[test]
fn progress_throttle_never_drops_a_fresh_incumbent() {
    let start = origin();
    let mut throttle = ProgressThrottle::new();

    assert!(throttle.should_send(start, false));

    // Well inside the interval: a heartbeat is dropped, an incumbent is not.
    // The strategies act on incumbents, so losing one to a rate limiter would
    // change what the solve produces, not just how often it reports.
    assert!(!throttle.should_send(start, false));
    assert!(throttle.should_send(start, true));
    assert!(throttle.should_send(start, true));
}

#[test]
fn progress_throttle_incumbent_resets_the_interval() {
    let start = origin();
    let mut throttle = ProgressThrottle::new();

    assert!(throttle.should_send(start, false));

    // An incumbent halfway through the interval is sent, and counts as the
    // last report: the conductor has just heard from us, so the heartbeat that
    // would have been due at `start + MIN_INTERVAL` is not.
    let half = start + ProgressThrottle::MIN_INTERVAL / 2;
    assert!(throttle.should_send(half, true));
    assert!(!throttle.should_send(start + ProgressThrottle::MIN_INTERVAL, false));
    assert!(throttle.should_send(half + ProgressThrottle::MIN_INTERVAL, false));
}
