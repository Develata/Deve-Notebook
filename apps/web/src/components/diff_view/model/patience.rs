use std::collections::HashMap;

/// 基于 Patience 锚点提取的行索引对。
///
/// Invariants:
/// - 返回的 `(old_idx, new_idx)` 严格递增。
/// - 仅包含在两侧均唯一出现的行。
pub fn anchors(old_lines: &[&str], new_lines: &[&str]) -> Vec<(usize, usize)> {
    let mut old_count: HashMap<&str, usize> = HashMap::new();
    let mut new_count: HashMap<&str, usize> = HashMap::new();
    for line in old_lines {
        *old_count.entry(*line).or_insert(0) += 1;
    }
    for line in new_lines {
        *new_count.entry(*line).or_insert(0) += 1;
    }

    let mut new_unique_pos: HashMap<&str, usize> = HashMap::new();
    for (idx, line) in new_lines.iter().enumerate() {
        if new_count.get(line).copied().unwrap_or(0) == 1 {
            new_unique_pos.insert(*line, idx);
        }
    }

    let mut candidates = Vec::new();
    for (idx, line) in old_lines.iter().enumerate() {
        if old_count.get(line).copied().unwrap_or(0) == 1
            && let Some(&new_idx) = new_unique_pos.get(line)
        {
            candidates.push((idx, new_idx));
        }
    }

    longest_increasing_by_new(&candidates)
}

fn longest_increasing_by_new(candidates: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut tails: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; candidates.len()];

    for i in 0..candidates.len() {
        let key = candidates[i].1;
        let pos = tails
            .binary_search_by(|&t| candidates[t].1.cmp(&key))
            .unwrap_or_else(|p| p);
        if pos > 0 {
            prev[i] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(i);
        } else {
            tails[pos] = i;
        }
    }

    let mut chain = Vec::new();
    let mut cur = tails.last().copied();
    while let Some(i) = cur {
        chain.push(candidates[i]);
        cur = prev[i];
    }
    chain.reverse();
    chain
}
