use std::collections::HashSet;

use crate::prefs::{DurationBucketConfig, GlobalPrefs};
use crate::yt::types::VideoDetails;

#[derive(Clone)]
pub struct DurationBucketState {
    pub config: DurationBucketConfig,
    pub selected: bool,
}

#[derive(Clone)]
pub struct DurationFilterState {
    pub allow_multiple: bool,
    pub buckets: Vec<DurationBucketState>,
}

impl DurationFilterState {
    pub fn from_global(global: &GlobalPrefs) -> Self {
        let mut state = Self {
            allow_multiple: global.duration_filters.allow_multiple,
            buckets: global
                .duration_filters
                .buckets
                .iter()
                .cloned()
                .map(|config| DurationBucketState {
                    selected: false,
                    config,
                })
                .collect(),
        };
        state.sync_from_ids(&global.active_duration_bucket_ids);
        state
    }

    pub fn sync_from_ids(&mut self, ids: &[String]) -> bool {
        let active: HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        let mut changed = false;
        for bucket in &mut self.buckets {
            let want = active.contains(bucket.config.id.as_str());
            if bucket.selected != want {
                bucket.selected = want;
                changed = true;
            }
        }
        changed |= self.ensure_minimum_selection();
        if !self.allow_multiple {
            changed |= self.enforce_single_selection();
        }
        changed
    }

    pub fn toggle(&mut self, id: &str) -> bool {
        let mut changed = false;
        if let Some(index) = self.buckets.iter().position(|b| b.config.id == id) {
            let is_catch_all = self.buckets[index].config.is_catch_all();
            if is_catch_all {
                let new_state = !self.buckets[index].selected;
                for bucket in &mut self.buckets {
                    let should_select = bucket.config.id == id && new_state;
                    if bucket.selected != should_select {
                        bucket.selected = should_select;
                        changed = true;
                    }
                }
            } else if self.allow_multiple {
                let new_state = !self.buckets[index].selected;
                if self.buckets[index].selected != new_state {
                    self.buckets[index].selected = new_state;
                    changed = true;
                }
                if new_state {
                    for bucket in &mut self.buckets {
                        if bucket.config.is_catch_all() && bucket.selected {
                            bucket.selected = false;
                            changed = true;
                        }
                    }
                } else {
                    changed |= self.ensure_default_if_empty();
                }
            } else {
                for (idx, bucket) in self.buckets.iter_mut().enumerate() {
                    let should_select = idx == index;
                    if bucket.selected != should_select {
                        bucket.selected = should_select;
                        changed = true;
                    }
                }
            }

            if !self.allow_multiple {
                changed |= self.enforce_single_selection();
            }
            changed |= self.ensure_minimum_selection();
        }
        changed
    }

    pub fn selected_ids(&self) -> Vec<String> {
        self.buckets
            .iter()
            .filter(|bucket| bucket.selected)
            .map(|bucket| bucket.config.id.clone())
            .collect()
    }

    pub fn allows(&self, secs: u64) -> bool {
        let mut any_active = false;
        for bucket in &self.buckets {
            if bucket.selected {
                any_active = true;
                if bucket.config.contains(secs) {
                    return true;
                }
            }
        }
        !any_active
    }

    fn ensure_minimum_selection(&mut self) -> bool {
        if self.buckets.iter().any(|bucket| bucket.selected) {
            return false;
        }

        if let Some((idx, _)) = self
            .buckets
            .iter()
            .enumerate()
            .find(|(_, bucket)| bucket.config.default_selected)
        {
            return self.select_only(idx);
        }

        self.activate_catch_all()
            || self
                .buckets
                .first_mut()
                .map(|bucket| {
                    if bucket.selected {
                        false
                    } else {
                        bucket.selected = true;
                        true
                    }
                })
                .unwrap_or(false)
    }

    fn enforce_single_selection(&mut self) -> bool {
        if self.allow_multiple {
            return false;
        }
        let mut found = false;
        let mut changed = false;
        for bucket in &mut self.buckets {
            if bucket.selected {
                if !found {
                    found = true;
                } else {
                    bucket.selected = false;
                    changed = true;
                }
            }
        }
        changed
    }

    fn ensure_default_if_empty(&mut self) -> bool {
        if self.buckets.iter().any(|bucket| bucket.selected) {
            return false;
        }
        self.activate_catch_all()
    }

    fn activate_catch_all(&mut self) -> bool {
        let mut found = false;
        let mut changed = false;
        for bucket in &mut self.buckets {
            let should_select = bucket.config.is_catch_all();
            if should_select {
                found = true;
            }
            if bucket.selected != should_select {
                bucket.selected = should_select;
                changed = true;
            }
        }
        if found {
            return changed;
        }

        if let Some(first) = self.buckets.first_mut() {
            if !first.selected {
                first.selected = true;
                return true;
            }
        }
        changed
    }

    fn select_only(&mut self, index: usize) -> bool {
        let mut changed = false;
        for (idx, bucket) in self.buckets.iter_mut().enumerate() {
            let should_select = idx == index;
            if bucket.selected != should_select {
                bucket.selected = should_select;
                changed = true;
            }
        }
        changed
    }
}

pub fn channel_sort_key(video: &VideoDetails) -> String {
    let preferred = video
        .channel_display_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    preferred
        .or_else(|| {
            let title = video.channel_title.trim();
            if title.is_empty() {
                None
            } else {
                Some(title.to_ascii_lowercase())
            }
        })
        .or_else(|| {
            let handle = video.channel_handle.trim();
            if handle.is_empty() {
                None
            } else {
                Some(handle.to_ascii_lowercase())
            }
        })
        .unwrap_or_default()
}
