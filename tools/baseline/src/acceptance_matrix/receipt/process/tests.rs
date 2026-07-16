use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn sleeping_child(isolated: bool) -> Child {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 30"]);
    if isolated {
        configure_process_group(&mut command);
    }
    command.spawn().unwrap()
}

#[test]
fn unix_child_group_is_isolated_before_group_signals() {
    let mut child = sleeping_child(true);
    let group = UnixProcessGroup::for_child(&child).unwrap();

    assert_eq!(group.id(), libc::pid_t::try_from(child.id()).unwrap());
    assert_ne!(group.id(), current_process_group().unwrap());
    terminate_owned_processes(&mut child).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).unwrap());
}

#[test]
fn unix_shared_parent_group_is_rejected() {
    let runner_group = current_process_group().unwrap();
    let mut child = sleeping_child(false);
    let error = terminate_owned_processes(&mut child).unwrap_err();

    assert!(error.to_string().contains("not an isolated child group"));
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).unwrap());
    assert_eq!(current_process_group().unwrap(), runner_group);
}

#[test]
fn unix_group_termination_does_not_leave_a_group_descendant_running() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let marker = std::env::temp_dir().join(format!(
        "deve-process-group-leak-{}-{unique}",
        std::process::id()
    ));
    let ready = marker.with_extension("ready");
    let mut command = Command::new("sh");
    command.args([
        "-c",
        "(sleep 1; printf leaked > \"$1\") & printf ready > \"$2\"; wait",
        "deve-process-group-test",
    ]);
    command.args([&marker, &ready]);
    configure_process_group(&mut command);
    let mut child = command.spawn().unwrap();

    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        ready.exists(),
        "descendant did not reach the readiness barrier"
    );
    terminate_owned_processes(&mut child).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).unwrap());
    thread::sleep(Duration::from_millis(1_200));
    assert!(!marker.exists(), "a timed-out descendant survived cleanup");
    let _ = fs::remove_file(marker);
    let _ = fs::remove_file(ready);
}
