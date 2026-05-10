use crate::storage::prefs::{read_i32_pref, write_i32_pref};

pub(crate) fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min, max)
}

pub(crate) fn read_width(key: &str) -> Option<i32> {
    read_i32_pref(key)
}

pub(crate) fn write_width(key: &str, value: i32) {
    let _ = write_i32_pref(key, value);
}
