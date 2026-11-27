//! Named sets of switches, and the one place they are written down.
//!
//! Before this, three `.cmd` files each carried a copy of the same
//! `if not defined VICTORY_...` table, and the copies had begun to disagree:
//! a different ambient, a different console model, one of them missing the
//! debug breadcrumbs, one of them unable to pass anything but a literal. A
//! preset replaces all of that with a name, `victory profiles` prints what the
//! name sets, and the launchers shrink to the four lines that matter.
//!
//! A preset never overwrites a switch that is already in the environment, the
//! way the `if not defined` lines never did: a one-off experiment
//! (`set VICTORY_AMBIENT=0.4`) still wins over the preset.
//!
//! The tables are data, not policy. What each switch does is in the comment
//! where the switch is read.

use std::collections::BTreeSet;

use clap::ValueEnum;

/// Which card a vendor-dependent preset is written for.
///
/// `Other` is the conservative set, not a fourth idea: a card we could not
/// name gets what NVIDIA gets, because the AMD-only switches lose faces and
/// road surfaces there while the reverse only costs a few milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum GpuVendor {
    /// Radeon: recorded lists and all four renderer ports are stable.
    Amd,
    /// GeForce: with recorded lists the heads and skin of people in play are
    /// not drawn, and `PORT_CBV` under 12 GB can drop road surfaces.
    Nvidia,
    /// A card that was not named: the conservative set.
    Other,
}

impl GpuVendor {
    /// The spelling on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Amd => "amd",
            Self::Nvidia => "nvidia",
            Self::Other => "other",
        }
    }
}

/// A named set of switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Profile {
    /// What `Play-VIctory.cmd` has always set: Series X, no recorded lists,
    /// ports 6 and 7 only, the constant ambient stand-in.
    ParetoResidency,
    /// What `Play-VIctory-AMD.cmd` has always set: the same plus recorded
    /// lists and all four renderer ports.
    ParetoResidencyAmd,
    /// 1080p output at two thirds render scale, 60 Hz frame tick, ports picked
    /// by vendor. The playable middle.
    Balanced,
    /// 936p, no frame cap, every port that is safe for the card.
    Speed,
    /// 1440p at scale 1.0 with the ray-tracing builder, so the title lights
    /// itself instead of the stand-in. About a third slower.
    Quality,
    /// 720p on the Series S profile for 4 GB cards.
    #[value(name = "lowvram")]
    LowVram,
}

impl Profile {
    /// The spelling on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::ParetoResidency => "pareto-residency",
            Self::ParetoResidencyAmd => "pareto-residency-amd",
            Self::Balanced => "balanced",
            Self::Speed => "speed",
            Self::Quality => "quality",
            Self::LowVram => "lowvram",
        }
    }

    /// The console model the title should see, unless `--console` says otherwise.
    pub fn console(self) -> &'static str {
        match self {
            Self::LowVram => "series-s",
            _ => "series-x",
        }
    }

    /// Every preset, in the order the help and `victory profiles` show them.
    pub fn all() -> Vec<Self> {
        vec![
            Self::ParetoResidency,
            Self::ParetoResidencyAmd,
            Self::Balanced,
            Self::Speed,
            Self::Quality,
            Self::LowVram,
        ]
    }

    /// The switches this preset sets, in the order they are written.
    ///
    /// No key appears twice: a preset that changes a base value replaces it,
    /// so the table can be read as the final state instead of a script.
    pub fn switches(self, vendor: GpuVendor) -> Vec<(&'static str, String)> {
        let mut table = Table::new();
        table.set("VICTORY_AMBIENT_AIR", "1.5");
        table.set("VICTORY_TEXTURE_NO_WAIT", "1");
        table.set("VICTORY_RELAY", "2");
        table.set("VICTORY_WIDE_BACK", "1");
        table.set("VICTORY_CARD_REHASH_RUNS", "1");
        table.set("VICTORY_JOIN_COPIES", "1");
        table.set("VICTORY_INDIRECT_WINDOW", "1");
        table.set("VICTORY_RELAY_COPY", "1");
        table.set("VICTORY_CARD_STEPS", "2");
        table.set("VICTORY_INDIRECT_BATCH", "2");
        table.set("VICTORY_MAP_THREADS", "1");
        table.set("VICTORY_CARD_AWAKE", "1");
        table.set("VICTORY_SHOW_AFTER", "1");
        table.set("VICTORY_TEXTURE_GUARD", "0");
        table.set("VICTORY_PORT_GUARD", "1");
        table.set("VICTORY_PORT_JOURNAL", "1");
        table.set("VICTORY_LIBRARY_STORE_LATER", "1");
        table.set("VICTORY_PIPELINE_WAIT_MS", "0");
        table.set("VICTORY_UPLOAD_AHEAD", "1");
        table.set("VICTORY_LEVELS_ON_DEMAND", "1");
        table.set("VICTORY_CARD_BUFFERS_MIN_KB", "64");
        table.set("VICTORY_LOG", "warn");
        // The pareto tables have always kept their own cache, apart from the
        // plain `victory` one: the residency work changed what is stored, and
        // sharing it would re-translate everything once.
        if let Some(dir) = default_cache_dir() {
            table.set("VICTORY_CACHE_DIR", dir);
        }

        match self {
            Self::ParetoResidency => {
                table.set("VICTORY_AMBIENT", "constant:0.8");
                table.set("VICTORY_THIN", "0");
                table.set("VICTORY_JOIN_PARTS", "0");
                table.set("VICTORY_PORT_CONSTANTS", "0");
                table.set("VICTORY_PORT_CBV", "0");
            }
            Self::ParetoResidencyAmd => {
                table.set("VICTORY_AMBIENT", "constant:0.8");
                table.set("VICTORY_THIN", "1");
                table.set("VICTORY_JOIN_PARTS", "8");
                table.set("VICTORY_PORT_CONSTANTS", "1");
                table.set("VICTORY_PORT_CBV", "1");
            }
            Self::Balanced => {
                table.set("VICTORY_AMBIENT", "constant:0.8");
                table.set("VICTORY_OUTPUT", "1920x1080");
                table.set("VICTORY_RENDER_SCALE", "0.667");
                // The rate the title asks for is 30 on Series X and it keeps to
                // it with a limiter of its own; this raises the ceiling. It does
                // not make frames - the submitting thread decides those.
                table.set("VICTORY_FRAME_INTERVAL_MS", "16.667");
                table.put_all(vendor_switches(vendor, "8"));
            }
            Self::Speed => {
                table.set("VICTORY_AMBIENT", "constant:0.8");
                table.set("VICTORY_OUTPUT", "1664x936");
                table.set("VICTORY_RENDER_SCALE", "0.667");
                table.set("VICTORY_FRAME_INTERVAL_MS", "0");
                table.put_all(vendor_switches(vendor, "8"));
            }
            Self::Quality => {
                // No VICTORY_AMBIENT here on purpose: with the builder running,
                // the title resolves its own light and a stand-in on top of it
                // is wrong, not merely redundant.
                table.set("VICTORY_OUTPUT", "2560x1440");
                table.set("VICTORY_RENDER_SCALE", "1.0");
                table.set("VICTORY_FRAME_INTERVAL_MS", "16.667");
                table.set("VICTORY_BVH_BUILDER", "1");
                table.set("VICTORY_PORT_RT", "1");
                table.set("VICTORY_THIN", "0");
                table.set("VICTORY_JOIN_PARTS", "0");
                table.set("VICTORY_PORT_CONSTANTS", "0");
                table.set("VICTORY_PORT_CBV", "0");
            }
            Self::LowVram => {
                table.set("VICTORY_AMBIENT", "0.8");
                table.set("VICTORY_OUTPUT", "1280x720");
                table.set("VICTORY_RENDER_SCALE", "0.667");
                table.set("VICTORY_THIN", "0");
                table.set("VICTORY_JOIN_PARTS", "0");
                table.set("VICTORY_PORT_CONSTANTS", "0");
                table.set("VICTORY_PORT_CBV", "0");
                table.set("VICTORY_CARD_BUFFERS_MIN_KB", "4096");
                table.set("VICTORY_CARD_BUFFERS_MB", "256");
                table.set("VICTORY_D3D12_BACKGROUND", "0");
            }
        }

        table.0
    }

    /// The switches to write, given what the environment already holds.
    ///
    /// A key that is already set is left alone: that is what the launchers'
    /// `if not defined` achieved, without the three copies.
    pub fn writes(
        self,
        vendor: GpuVendor,
        is_set: impl Fn(&str) -> bool,
    ) -> Vec<(&'static str, String)> {
        self.switches(vendor)
            .into_iter()
            .filter(|(k, _)| !is_set(k))
            .collect()
    }

    /// Write [`Self::writes`] into the process environment.
    pub fn apply(self, vendor: GpuVendor) -> Vec<&'static str> {
        let existing: BTreeSet<String> = std::env::vars_os()
            .filter_map(|(k, _)| k.into_string().ok())
            .collect();
        let chosen = self.writes(vendor, |k| existing.contains(k));
        let mut wrote = Vec::with_capacity(chosen.len());
        for (k, v) in chosen {
            // SAFETY: called from `main` before any other thread exists, and
            // before anything reads a switch.
            unsafe { std::env::set_var(k, v) };
            wrote.push(k);
        }
        wrote
    }
}

/// The four switches a vendor decides, in one place so the presets cannot
/// disagree about them.
fn vendor_switches(
    vendor: GpuVendor,
    join_parts: &'static str,
) -> [(&'static str, String); 4] {
    match vendor {
        GpuVendor::Amd => [
            ("VICTORY_THIN", "1".to_owned()),
            ("VICTORY_JOIN_PARTS", join_parts.to_owned()),
            ("VICTORY_PORT_CONSTANTS", "1".to_owned()),
            ("VICTORY_PORT_CBV", "1".to_owned()),
        ],
        GpuVendor::Nvidia | GpuVendor::Other => [
            ("VICTORY_THIN", "0".to_owned()),
            ("VICTORY_JOIN_PARTS", "0".to_owned()),
            ("VICTORY_PORT_CONSTANTS", "0".to_owned()),
            ("VICTORY_PORT_CBV", "0".to_owned()),
        ],
    }
}

/// `%LOCALAPPDATA%\VIctory\cache-pareto`, the cache the pareto launchers use.
///
/// `None` where there is no such directory (a test run on a machine without
/// the variable): the key is then not set at all, and `Config` keeps its own
/// default rather than an empty path.
fn default_cache_dir() -> Option<String> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let base = base.to_string_lossy();
    let trimmed = base.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}\\VIctory\\cache-pareto"))
}

/// A preset's table: writing a key that is already there replaces it in place,
/// so no key can appear twice however a preset is built up.
struct Table(Vec<(&'static str, String)>);

impl Table {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn set(&mut self, key: &'static str, value: impl Into<String>) {
        self.0.retain(|(k, _)| *k != key);
        self.0.push((key, value.into()));
    }

    fn put_all(&mut self, pairs: [(&'static str, String); 4]) {
        for (k, v) in pairs {
            self.set(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(p: Profile, v: GpuVendor) -> Vec<&'static str> {
        p.switches(v).into_iter().map(|(k, _)| k).collect()
    }

    fn value(p: Profile, v: GpuVendor, key: &str) -> Option<String> {
        p.switches(v)
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, got)| got)
    }

    fn has(p: Profile, v: GpuVendor, key: &str, want: &str) -> bool {
        value(p, v, key).as_deref() == Some(want)
    }

    #[test]
    fn names_match_the_command_line_spellings() {
        for p in Profile::all() {
            let spelled = Profile::value_variants()
                .iter()
                .find(|q| **q == p)
                .and_then(|q| q.to_possible_value())
                .map(|v| v.get_name().to_owned());
            assert_eq!(spelled.as_deref(), Some(p.name()), "{p:?}");
        }
        for v in [GpuVendor::Amd, GpuVendor::Nvidia, GpuVendor::Other] {
            let spelled = GpuVendor::value_variants()
                .iter()
                .find(|q| **q == v)
                .and_then(|q| q.to_possible_value())
                .map(|x| x.get_name().to_owned());
            assert_eq!(spelled.as_deref(), Some(v.name()), "{v:?}");
        }
    }

    #[test]
    fn every_preset_names_every_key_once() {
        for p in Profile::all() {
            for vendor in [GpuVendor::Amd, GpuVendor::Nvidia, GpuVendor::Other] {
                let all = keys(p, vendor);
                let unique: BTreeSet<&str> = all.iter().copied().collect();
                assert_eq!(all.len(), unique.len(), "{:?} / {vendor:?} repeats a key", p.name());
            }
        }
    }

    #[test]
    fn the_old_launchers_keep_their_tables() {
        // Play-VIctory.cmd: Series X, no recorded lists, ports 6 and 7 only.
        assert_eq!(Profile::ParetoResidency.console(), "series-x");
        assert!(has(Profile::ParetoResidency, GpuVendor::Other, "VICTORY_THIN", "0"));
        assert!(has(Profile::ParetoResidency, GpuVendor::Other, "VICTORY_JOIN_PARTS", "0"));
        assert!(has(Profile::ParetoResidency, GpuVendor::Other, "VICTORY_PORT_CBV", "0"));
        assert!(has(Profile::ParetoResidency, GpuVendor::Other, "VICTORY_AMBIENT", "constant:0.8"));
        assert!(has(Profile::ParetoResidency, GpuVendor::Other, "VICTORY_PORT_GUARD", "1"));

        // Play-VIctory-AMD.cmd: the same, plus recorded lists and all four ports.
        assert!(has(Profile::ParetoResidencyAmd, GpuVendor::Amd, "VICTORY_THIN", "1"));
        assert!(has(Profile::ParetoResidencyAmd, GpuVendor::Amd, "VICTORY_JOIN_PARTS", "8"));
        assert!(has(Profile::ParetoResidencyAmd, GpuVendor::Amd, "VICTORY_PORT_CONSTANTS", "1"));
        assert!(has(Profile::ParetoResidencyAmd, GpuVendor::Amd, "VICTORY_PORT_CBV", "1"));

        // Play-VIctory-4GB.cmd: Series S, 720p, coarser card-buffer granularity.
        assert_eq!(Profile::LowVram.console(), "series-s");
        assert!(has(Profile::LowVram, GpuVendor::Other, "VICTORY_OUTPUT", "1280x720"));
        assert!(has(Profile::LowVram, GpuVendor::Other, "VICTORY_CARD_BUFFERS_MIN_KB", "4096"));
        assert!(has(Profile::LowVram, GpuVendor::Other, "VICTORY_D3D12_BACKGROUND", "0"));
    }

    #[test]
    fn the_vendor_decides_the_risky_four() {
        for p in [Profile::Balanced, Profile::Speed] {
            assert!(has(p, GpuVendor::Amd, "VICTORY_THIN", "1"), "{:?}", p.name());
            assert!(has(p, GpuVendor::Amd, "VICTORY_PORT_CBV", "1"));
            assert!(has(p, GpuVendor::Nvidia, "VICTORY_THIN", "0"));
            assert!(has(p, GpuVendor::Nvidia, "VICTORY_PORT_CBV", "0"));
            // An unnamed card is treated as NVIDIA: the safe direction.
            assert_eq!(p.switches(GpuVendor::Nvidia), p.switches(GpuVendor::Other));
        }
    }

    #[test]
    fn quality_hands_the_light_back_to_the_title() {
        let sw = Profile::Quality.switches(GpuVendor::Other);
        assert!(!sw.iter().any(|(k, _)| *k == "VICTORY_AMBIENT"));
        assert!(has(Profile::Quality, GpuVendor::Other, "VICTORY_BVH_BUILDER", "1"));
        assert!(has(Profile::Quality, GpuVendor::Other, "VICTORY_PORT_RT", "1"));
        // The other presets keep the stand-in, in one form or the other.
        for p in [Profile::Balanced, Profile::Speed] {
            assert!(has(p, GpuVendor::Other, "VICTORY_AMBIENT", "constant:0.8"));
        }
        assert!(has(Profile::LowVram, GpuVendor::Other, "VICTORY_AMBIENT", "0.8"));
    }

    #[test]
    fn nothing_is_written_over() {
        let p = Profile::Balanced;
        let everything: BTreeSet<String> = p
            .switches(GpuVendor::Other)
            .into_iter()
            .map(|(k, _)| k.to_owned())
            .collect();
        assert!(p.writes(GpuVendor::Other, |k| everything.contains(k)).is_empty());

        let first = p.switches(GpuVendor::Other)[0].0;
        let one = BTreeSet::from([first.to_owned()]);
        let written = p.writes(GpuVendor::Other, |k| one.contains(k));
        assert_eq!(written.len() + 1, p.switches(GpuVendor::Other).len());
        assert!(!written.iter().any(|(k, _)| *k == first));
    }

    #[test]
    fn the_cache_stays_apart_from_the_plain_one() {
        // Only where there is a %LOCALAPPDATA% to expand; a test machine
        // without it just does not get the key, which is also correct.
        if std::env::var_os("LOCALAPPDATA").is_some() {
            let sw = Profile::ParetoResidency.switches(GpuVendor::Other);
            let cache = sw
                .iter()
                .find(|(k, _)| *k == "VICTORY_CACHE_DIR")
                .map(|(_, v)| v.clone());
            let cache = cache.expect("VICTORY_CACHE_DIR is set when LOCALAPPDATA is");
            assert!(cache.ends_with("VIctory\\cache-pareto"), "{cache}");
        }
    }
}
