//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!   - 16_ai_agent#native-ai-chat-runtime

use super::NativeAiProviderSettingsRuntime;
use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex, OnceLock};

struct Slot {
    next_generation: u64,
    current: Option<(u64, Arc<NativeAiProviderSettingsRuntime>)>,
}

static SLOT: OnceLock<Mutex<Slot>> = OnceLock::new();

pub(crate) struct ProviderSettingsRegistration {
    generation: u64,
}

pub(crate) fn register(
    runtime: Arc<NativeAiProviderSettingsRuntime>,
) -> Result<ProviderSettingsRegistration> {
    let mut slot = SLOT
        .get_or_init(|| {
            Mutex::new(Slot {
                next_generation: 1,
                current: None,
            })
        })
        .lock()
        .map_err(|_| anyhow!("AI provider registry lock poisoned"))?;
    let generation = slot.next_generation;
    slot.next_generation = slot
        .next_generation
        .checked_add(1)
        .ok_or_else(|| anyhow!("AI provider registry generation exhausted"))?;
    slot.current = Some((generation, runtime));
    Ok(ProviderSettingsRegistration { generation })
}

pub(crate) fn current() -> Result<Arc<NativeAiProviderSettingsRuntime>> {
    SLOT.get_or_init(|| {
        Mutex::new(Slot {
            next_generation: 1,
            current: None,
        })
    })
    .lock()
    .map_err(|_| anyhow!("AI provider registry lock poisoned"))?
    .current
    .as_ref()
    .map(|(_, runtime)| runtime.clone())
    .ok_or_else(|| anyhow!("Native AI provider settings are not registered"))
}

impl Drop for ProviderSettingsRegistration {
    fn drop(&mut self) {
        if let Ok(mut slot) = SLOT
            .get_or_init(|| {
                Mutex::new(Slot {
                    next_generation: 1,
                    current: None,
                })
            })
            .lock()
            && slot.current.as_ref().map(|(generation, _)| *generation) == Some(self.generation)
        {
            slot.current = None;
        }
    }
}
