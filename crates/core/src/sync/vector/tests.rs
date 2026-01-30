// crates\core\src\sync\vector
//! # 版本向量单元测试

use super::*;

/// 辅助函数：快速创建 PeerId
fn p(id: &str) -> PeerId {
    PeerId::new(id)
}

#[test]
fn test_vector_update_merge() {
    let mut v1 = VersionVector::new();
    v1.update(p("A"), 10);
    v1.update(p("B"), 5);

    assert_eq!(v1.get(&p("A")), 10);
    assert_eq!(v1.get(&p("B")), 5);
    assert_eq!(v1.get(&p("C")), 0);

    let mut v2 = VersionVector::new();
    v2.update(p("B"), 10);
    v2.update(p("C"), 7);

    v1.merge(&v2);

    assert_eq!(v1.get(&p("A")), 10);
    assert_eq!(v1.get(&p("B")), 10); // Updated to max
    assert_eq!(v1.get(&p("C")), 7); // New entry
}

#[test]
fn test_diff_scenarios() {
    let mut local = VersionVector::new();
    let mut remote = VersionVector::new();

    // Scenario 1: A leading (Local has more data from A)
    local.update(p("A"), 10);
    remote.update(p("A"), 5);

    let (missing_remote, missing_local) = local.diff(&remote);
    assert!(!missing_remote.is_empty());
    assert!(missing_local.is_empty());
    let (id, range) = &missing_remote[0];
    assert_eq!(id, &p("A"));
    assert_eq!(range, &(6..11)); // Expecting 6,7,8,9,10.

    // Scenario 2: B leading (Remote has more data from B)
    local.update(p("B"), 3);
    remote.update(p("B"), 8);

    let (missing_remote, missing_local) = local.diff(&remote);

    // Check missing from remote (A case)
    assert!(missing_remote
        .iter()
        .any(|(id, r)| id == &p("A") && r == &(6..11)));

    // Check missing from local (B case)
    assert!(missing_local
        .iter()
        .any(|(id, r)| id == &p("B") && r == &(4..9)));
}

#[test]
fn test_diff_concurrent() {
    // 分叉/并发场景
    // Local: { A: 10, B: 20 }
    // Remote: { A: 12, B: 15 }
    let mut local = VersionVector::new();
    local.update(p("A"), 10);
    local.update(p("B"), 20);

    let mut remote = VersionVector::new();
    remote.update(p("A"), 12);
    remote.update(p("B"), 15);

    let (missing_remote, missing_local) = local.diff(&remote);

    // Remote needs B: 16..21
    assert!(missing_remote
        .iter()
        .any(|(id, r)| id == &p("B") && r == &(16..21)));

    // Local needs A: 11..13
    assert!(missing_local
        .iter()
        .any(|(id, r)| id == &p("A") && r == &(11..13)));
}

#[test]
fn test_merge_idempotent() {
    // 验证幂等性: v.merge(&v) == v
    let mut v = VersionVector::new();
    v.update(p("A"), 5);
    v.update(p("B"), 10);

    let v_clone = v.clone();
    v.merge(&v_clone);

    assert_eq!(v, v_clone);
}

#[test]
fn test_intersection() {
    let mut v1 = VersionVector::new();
    v1.update(p("A"), 10);
    v1.update(p("B"), 5);

    let mut v2 = VersionVector::new();
    v2.update(p("A"), 7);
    v2.update(p("B"), 8);
    v2.update(p("C"), 3);

    let lca = v1.intersection(&v2);

    assert_eq!(lca.get(&p("A")), 7); // min(10, 7)
    assert_eq!(lca.get(&p("B")), 5); // min(5, 8)
    assert_eq!(lca.get(&p("C")), 0); // min(0, 3) = 0, not stored
}
