//! Foreground cache and activation rules from Remapcon src/app/main.cpp (MIT).
#![forbid(unsafe_code)]

use std::time::Duration;

#[derive(Default)]
pub struct ForegroundCache {
    previous: Option<(usize, u64, Duration, bool)>,
}
impl ForegroundCache {
    pub fn check(
        &mut self,
        window: usize,
        target_version: u64,
        now: Duration,
        query: impl FnOnce() -> bool,
    ) -> bool {
        if let Some((last_window, version, checked, result)) = self.previous {
            if window == last_window
                && target_version == version
                && now.saturating_sub(checked) < Duration::from_millis(100)
            {
                return result;
            }
        }
        let result = query();
        self.previous = Some((window, target_version, now, result));
        result
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Activation {
    pub hold_controller: bool,
    pub take_from_steam: bool,
    pub output: bool,
}
pub fn activation(
    requested: bool,
    auto_mode: bool,
    preview: bool,
    target_foreground: bool,
    main_foreground: bool,
    steam_takeover: bool,
    elevated: bool,
) -> Activation {
    let automatic = auto_mode && target_foreground;
    let preview_active = preview && main_foreground;
    Activation {
        hold_controller: requested || automatic || preview_active,
        take_from_steam: steam_takeover && elevated && (automatic || preview_active),
        output: target_foreground && !preview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_and_activation_preserve_target_changes_and_manual_preview_rules() {
        let mut cache = ForegroundCache::default();
        assert!(cache.check(1, 0, Duration::ZERO, || true));
        assert!(cache.check(1, 0, Duration::from_millis(99), || panic!("cached")));
        assert!(!cache.check(1, 0, Duration::from_millis(100), || false));
        assert!(cache.check(2, 0, Duration::from_millis(101), || true));
        assert!(!cache.check(2, 1, Duration::from_millis(102), || false));
        assert!(!cache.check(2, 1, Duration::from_millis(103), || panic!(
            "cache failure too"
        )));
        assert_eq!(
            activation(true, false, false, false, false, true, true),
            Activation {
                hold_controller: true,
                take_from_steam: false,
                output: false,
            }
        );
        assert_eq!(
            activation(false, true, false, true, false, true, true),
            Activation {
                hold_controller: true,
                take_from_steam: true,
                output: true,
            }
        );
        assert!(!activation(false, true, false, true, false, true, false).take_from_steam);
        assert_eq!(
            activation(false, false, true, true, true, true, true),
            Activation {
                hold_controller: true,
                take_from_steam: true,
                output: false,
            }
        );
        assert!(!activation(false, true, false, false, true, true, true).hold_controller);
    }
}
