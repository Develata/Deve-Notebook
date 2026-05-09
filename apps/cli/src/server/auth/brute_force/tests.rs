//! plan_ref:
//!   - 09_auth#auth-rate-limiting
//!
//! Brute force guard tests.

use super::*;

fn ip(last: u8) -> IpAddr {
    format!("10.0.0.{last}").parse().unwrap()
}

fn record_failures(guard: &BruteForceGuard, ip: &IpAddr, failures: u32) {
    for _ in 0..failures {
        guard.record_failure(ip);
    }
}

#[test]
fn test_not_blocked_initially() {
    let guard = BruteForceGuard::new();
    assert!(!guard.is_blocked(&ip(1)));
}

#[test]
fn test_blocked_after_max_failures() {
    let guard = BruteForceGuard::new();
    let ip = ip(2);
    record_failures(&guard, &ip, MAX_FAILURES);
    assert!(guard.is_blocked(&ip));
}

#[test]
fn test_cleared_on_success() {
    let guard = BruteForceGuard::new();
    let ip = ip(3);
    record_failures(&guard, &ip, MAX_FAILURES);
    assert!(guard.is_blocked(&ip));
    guard.record_success(&ip);
    assert!(!guard.is_blocked(&ip));
}

#[test]
fn test_four_failures_not_blocked() {
    let guard = BruteForceGuard::new();
    let ip = ip(4);
    record_failures(&guard, &ip, MAX_FAILURES - 1);
    assert!(!guard.is_blocked(&ip));
}

#[test]
fn poisoned_lock_blocks_ip_fail_closed() {
    let guard = BruteForceGuard::new();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = guard.records.lock().expect("lock");
        panic!("poison brute force guard");
    }));
    assert!(guard.is_blocked(&ip(5)));
}
