//! plan_ref:
//!   - 08_auth#auth-rate-limiting
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
        let _guard = guard.state.lock().expect("lock");
        panic!("poison brute force guard");
    }));
    assert!(guard.is_blocked(&ip(5)));
}

#[test]
fn expired_ban_resets_failure_count_before_new_attempt() {
    let guard = BruteForceGuard::new();
    let ip = ip(6);
    record_failures(&guard, &ip, MAX_FAILURES);
    {
        let mut state = guard.state.lock().expect("lock state");
        let expired_at = Instant::now()
            .checked_sub(BAN_DURATION + Duration::from_secs(1))
            .expect("representable expired instant");
        state
            .records
            .get_mut(&ip)
            .expect("blocked record")
            .expires_at = expired_at;
    }

    assert!(!guard.is_blocked(&ip));
    guard.record_failure(&ip);

    let state = guard.state.lock().expect("lock state");
    assert_eq!(state.records[&ip].failures, 1);
    drop(state);
    assert!(!guard.is_blocked(&ip));
}

#[test]
fn brute_force_capacity_fails_closed_for_new_ip() {
    let guard = BruteForceGuard::with_capacity(2);
    guard.record_failure(&ip(7));
    guard.record_failure(&ip(8));

    assert!(!guard.is_blocked(&ip(7)));
    assert!(guard.is_blocked(&ip(9)));
    let state = guard.state.lock().expect("state");
    assert_eq!(state.records.len(), 2);
    assert!(state.capacity_warning_emitted);
}

#[test]
fn brute_force_reclaims_expired_capacity() {
    let guard = BruteForceGuard::with_capacity(1);
    let expired_ip = ip(10);
    let replacement_ip = ip(11);
    guard.record_failure(&expired_ip);
    {
        let mut state = guard.state.lock().expect("state");
        let expired_at = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .expect("representable expired instant");
        state
            .records
            .get_mut(&expired_ip)
            .expect("record")
            .expires_at = expired_at;
    }

    assert!(!guard.is_blocked(&replacement_ip));
    guard.record_failure(&replacement_ip);

    let state = guard.state.lock().expect("state");
    assert_eq!(state.records.len(), 1);
    assert!(state.records.contains_key(&replacement_ip));
    assert!(!state.capacity_warning_emitted);
}
