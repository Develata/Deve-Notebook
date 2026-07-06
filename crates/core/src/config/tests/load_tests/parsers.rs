use crate::config::{AppProfile, MergeStrategy, SyncMode};

#[test]
fn runtime_config_value_parsers_reject_unknown_values() {
    assert!("standard".parse::<AppProfile>().is_ok());
    assert!("low-spec".parse::<AppProfile>().is_ok());
    assert!("debug".parse::<AppProfile>().is_err());

    assert!("auto".parse::<SyncMode>().is_ok());
    assert!("manual".parse::<SyncMode>().is_ok());
    assert!("strict".parse::<SyncMode>().is_err());

    assert!("manual".parse::<MergeStrategy>().is_ok());
    assert!("auto".parse::<MergeStrategy>().is_ok());
    assert!("crdt".parse::<MergeStrategy>().is_err());
}
