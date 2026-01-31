use ropey::Rope;

pub(super) struct Utf16IndexCache {
    checkpoints: Vec<(u32, usize)>,
    step: u32,
}

impl Utf16IndexCache {
    pub(super) fn new(step: u32) -> Self {
        let step = step.max(8);
        Self {
            checkpoints: vec![(0, 0)],
            step,
        }
    }

    pub(super) fn build(rope: &Rope, step: u32) -> Self {
        let mut cache = Self::new(step);
        cache.rebuild(rope);
        cache
    }

    pub(super) fn step(&self) -> u32 {
        self.step
    }

    pub(super) fn locate(&self, rope: &Rope, utf16_index: u32) -> usize {
        if utf16_index == 0 {
            return 0;
        }

        let (base_utf16, base_char) = match self
            .checkpoints
            .binary_search_by(|(u, _)| u.cmp(&utf16_index))
        {
            Ok(idx) => return self.checkpoints[idx].1,
            Err(0) => (0u32, 0usize),
            Err(idx) => self.checkpoints[idx - 1],
        };

        let mut remaining = (utf16_index - base_utf16) as usize;
        if remaining == 0 {
            return base_char;
        }

        let slice = rope.slice(base_char..);
        let mut char_idx = base_char;
        for chunk in slice.chunks() {
            for ch in chunk.chars() {
                let next = ch.len_utf16();
                if remaining <= next {
                    return char_idx + 1;
                }
                remaining -= next;
                char_idx += 1;
            }
        }

        rope.len_chars()
    }

    pub(super) fn update_after_insert(
        &mut self,
        pos: u32,
        utf16_delta: u32,
        char_delta: usize,
    ) -> bool {
        if utf16_delta >= self.step {
            return true;
        }

        for (u, c) in &mut self.checkpoints {
            if *u >= pos {
                *u += utf16_delta;
                *c += char_delta;
            }
        }
        false
    }

    pub(super) fn update_after_delete(
        &mut self,
        pos: u32,
        len: u32,
        utf16_delta: u32,
        char_delta: usize,
    ) -> bool {
        if utf16_delta >= self.step {
            return true;
        }

        let end = pos.saturating_add(len);
        for (u, c) in &mut self.checkpoints {
            if *u >= end {
                *u = u.saturating_sub(utf16_delta);
                *c = c.saturating_sub(char_delta);
            } else if *u >= pos {
                *u = pos;
                *c = c.saturating_sub(char_delta.min(*c));
            }
        }
        false
    }

    pub(super) fn rebuild(&mut self, rope: &Rope) {
        self.checkpoints.clear();
        self.checkpoints.push((0, 0));

        let mut utf16 = 0u32;
        let mut char_idx = 0usize;
        let step = self.step;

        for chunk in rope.chunks() {
            for ch in chunk.chars() {
                let next = ch.len_utf16() as u32;
                if utf16 % step == 0 {
                    self.checkpoints.push((utf16, char_idx));
                }
                utf16 += next;
                char_idx += 1;
            }
        }
    }
}
