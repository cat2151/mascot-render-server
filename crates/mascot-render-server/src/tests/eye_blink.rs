use std::time::{Duration, Instant};

use crate::eye_blink::EyeBlinkLoop;

#[test]
fn eye_blink_loop_waits_random_open_interval_then_closes() {
    let now = Instant::now();
    let mut blink = EyeBlinkLoop::new_for_test(now, 1);
    let first_deadline = blink.current_deadline_for_test();

    assert!(first_deadline >= now + Duration::from_millis(1000));
    assert!(first_deadline <= now + Duration::from_millis(8000));
    assert!(!blink.is_closed(first_deadline - Duration::from_millis(1)));
    assert!(blink.is_closed(first_deadline));
}

#[test]
fn eye_blink_loop_reopens_after_two_hundred_milliseconds() {
    let now = Instant::now();
    let mut blink = EyeBlinkLoop::new_for_test(now, 2);
    let first_deadline = blink.current_deadline_for_test();

    assert!(blink.is_closed(first_deadline));
    assert!(!blink.is_closed(first_deadline + Duration::from_millis(200)));
}

#[test]
fn eye_blink_loop_elapsed_offset_shifts_initial_deadline() {
    let now = Instant::now();
    let blink = EyeBlinkLoop::new_with_seed_and_elapsed(now, 7, Duration::ZERO);
    let shifted_blink = EyeBlinkLoop::new_with_seed_and_elapsed(now, 7, Duration::from_millis(500));

    assert_eq!(
        shifted_blink.current_deadline_for_test() + Duration::from_millis(500),
        blink.current_deadline_for_test()
    );
}
