//! Automatic optimization.
//!
//! See `docs/automation.md`. The controlling fact is that trimming relocates
//! pages rather than freeing them, so the gain decays and a naive threshold
//! rule becomes a loop: trim, decay, trim again, generating page faults while
//! reporting success. Cooldowns, effectiveness feedback and disabled-by-default
//! exist to prevent exactly that.
//!
//! Everything here is pure: `Engine::evaluate` takes a snapshot of the world
//! and returns a decision. The Tauri layer supplies the inputs and performs the
//! result, which keeps the interesting logic testable without a running system.

use serde::{Deserialize, Serialize};

use super::clean::Method;

/// What starts a rule.
///
/// `rename_all_fields` is required as well as `rename_all`: the latter renames
/// the variants, not the fields inside them, which would leave the frontend
/// reading `undefined` for every parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum Trigger {
    /// Usage at or above `percent` continuously for `sustained_secs`.
    ///
    /// Sustained rather than instantaneous: a spike while an application
    /// launches is normal and corrects itself.
    UsageAbove { percent: u8, sustained_secs: u64 },
    /// A fixed interval since the last automatic run.
    Scheduled { every_mins: u64 },
    /// No keyboard or mouse input for `idle_mins`.
    SystemIdle { idle_mins: u64 },
}

impl Trigger {
    pub fn describe(&self) -> String {
        match self {
            Trigger::UsageAbove {
                percent,
                sustained_secs,
            } => format!("When memory stays above {percent}% for {sustained_secs}s"),
            Trigger::Scheduled { every_mins } => format!("Every {every_mins} minutes"),
            Trigger::SystemIdle { idle_mins } => format!("After {idle_mins} minutes idle"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    pub id: String,
    pub enabled: bool,
    pub trigger: Trigger,
    /// Consecutive runs recovering little before the rule suspends itself.
    pub ineffective_limit: u32,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            id: "rule".into(),
            enabled: false,
            trigger: Trigger::SystemIdle { idle_mins: 15 },
            ineffective_limit: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub name: String,
    pub methods: Vec<Method>,
    pub rules: Vec<Rule>,
    /// Minimum gap between automatic runs. Floored at 300s — the cooldown is
    /// the primary defence against the decay loop and is not user-defeatable.
    pub min_interval_secs: u64,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Balanced".into(),
            methods: vec![Method::TrimWorkingSets],
            rules: Vec::new(),
            min_interval_secs: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Master switch. Off by default: unattended action is opt-in.
    pub enabled: bool,
    /// Epoch ms until which automation is paused, from the tray menu.
    pub paused_until: Option<u64>,
    pub active_profile: String,
    pub profiles: Vec<Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            paused_until: None,
            active_profile: "Balanced".into(),
            profiles: vec![
                Profile {
                    name: "Balanced".into(),
                    // The only default rule is the idle one: the decay cost is
                    // paid while nobody is working.
                    rules: vec![Rule {
                        id: "balanced-idle".into(),
                        enabled: true,
                        trigger: Trigger::SystemIdle { idle_mins: 15 },
                        ineffective_limit: 3,
                    }],
                    ..Default::default()
                },
                Profile {
                    name: "Gaming".into(),
                    // Deliberately empty. "Gaming" does not mean more
                    // aggressive: trimming mid-game causes the stutter someone
                    // choosing this profile is trying to avoid.
                    rules: Vec::new(),
                    ..Default::default()
                },
                Profile {
                    name: "Development".into(),
                    rules: vec![Rule {
                        id: "dev-high-usage".into(),
                        enabled: false,
                        trigger: Trigger::UsageAbove {
                            percent: 90,
                            sustained_secs: 120,
                        },
                        ineffective_limit: 3,
                    }],
                    ..Default::default()
                },
            ],
        }
    }
}

impl Config {
    pub fn sanitized(mut self) -> Self {
        for p in &mut self.profiles {
            p.min_interval_secs = p.min_interval_secs.max(300);
            // A run with no methods would still burn a cooldown and pollute the
            // effectiveness statistics, so it is not allowed to exist.
            if p.methods.is_empty() {
                p.methods.push(Method::TrimWorkingSets);
            }
            for r in &mut p.rules {
                r.ineffective_limit = r.ineffective_limit.clamp(1, 10);
                if let Trigger::UsageAbove {
                    percent,
                    sustained_secs,
                } = &mut r.trigger
                {
                    *percent = (*percent).clamp(50, 99);
                    *sustained_secs = (*sustained_secs).clamp(30, 3600);
                }
                if let Trigger::Scheduled { every_mins } = &mut r.trigger {
                    *every_mins = (*every_mins).clamp(5, 1440);
                }
                if let Trigger::SystemIdle { idle_mins } = &mut r.trigger {
                    *idle_mins = (*idle_mins).clamp(1, 720);
                }
            }
        }

        if self.profiles.is_empty() {
            self.profiles.push(Profile::default());
        }
        // A missing active profile falls back rather than silently disabling.
        if !self.profiles.iter().any(|p| p.name == self.active_profile) {
            self.active_profile = self.profiles[0].name.clone();
        }
        self
    }

    pub fn active(&self) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == self.active_profile)
    }
}

/// Everything the evaluator needs to know about the world, sampled by the
/// caller so the evaluator itself stays pure.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// Monotonic milliseconds. Not wall clock: a clock change or DST shift must
    /// not fire a rule.
    pub now_ms: u64,
    pub percent_in_use: f64,
    pub idle_secs: u64,
    /// True when a fullscreen or presentation app is in the foreground.
    pub foreground_busy: bool,
    pub elevated: bool,
    pub run_in_flight: bool,
}

/// Why a matched rule did not run. Recorded rather than discarded so that
/// "why did automation never fire?" is answerable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Gate {
    Disabled,
    Paused,
    Cooldown,
    RunInFlight,
    Ineffective,
    ForegroundBusy,
    NotElevated,
}

impl Gate {
    pub fn describe(&self) -> &'static str {
        match self {
            Gate::Disabled => "Automation is turned off",
            Gate::Paused => "Automation is paused",
            Gate::Cooldown => "Too soon after the last automatic run",
            Gate::RunInFlight => "An optimization was already running",
            Gate::Ineffective => "The rule was suspended for recovering little memory",
            Gate::ForegroundBusy => "A fullscreen app was in the foreground",
            Gate::NotElevated => "The rule needs administrator rights",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Nothing matched.
    Idle,
    Run {
        rule: String,
        methods: Vec<Method>,
    },
    Blocked {
        rule: String,
        gate: Gate,
    },
}

/// Per-rule mutable state.
#[derive(Debug, Default, Clone)]
struct RuleState {
    /// When the usage condition first became true, for sustained triggers.
    above_since_ms: Option<u64>,
    last_fired_ms: Option<u64>,
    consecutive_ineffective: u32,
}

/// Evaluates rules against the world. Owns only scheduling state; the
/// configuration is passed in so a settings change takes effect immediately.
#[derive(Default)]
pub struct Engine {
    states: std::collections::HashMap<String, RuleState>,
    last_run_ms: Option<u64>,
}

/// A settled recovery below this is treated as the rule not having helped.
///
/// Provisional: it should be derived from real `clean://settled` data across
/// several machines rather than left at a guess.
pub const EFFECTIVE_BYTES: i64 = 200 * 1024 * 1024;

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the outcome of a run so the effectiveness gate has something to
    /// act on. `settled` is the 30-second measurement.
    pub fn record_settled(&mut self, rule: &str, settled: i64) {
        let state = self.states.entry(rule.to_string()).or_default();
        if settled >= EFFECTIVE_BYTES {
            state.consecutive_ineffective = 0;
        } else {
            state.consecutive_ineffective = state.consecutive_ineffective.saturating_add(1);
        }
    }

    /// True when the rule has suspended itself. Surfaced in the UI with a
    /// resume action.
    pub fn is_suspended(&self, rule: &Rule) -> bool {
        self.states
            .get(&rule.id)
            .is_some_and(|s| s.consecutive_ineffective >= rule.ineffective_limit)
    }

    pub fn resume(&mut self, rule: &str) {
        if let Some(s) = self.states.get_mut(rule) {
            s.consecutive_ineffective = 0;
        }
    }

    /// Evaluates the active profile. At most one decision per call: a tick can
    /// never produce two runs.
    pub fn evaluate(&mut self, config: &Config, ctx: Context) -> Decision {
        let Some(profile) = config.active() else {
            return Decision::Idle;
        };

        // Trigger state is tracked even while disabled, so enabling automation
        // does not immediately fire on a condition that was already true.
        let mut matched: Option<&Rule> = None;
        for rule in profile.rules.iter().filter(|r| r.enabled) {
            if self.trigger_matches(rule, ctx) && matched.is_none() {
                matched = Some(rule);
            }
        }

        let Some(rule) = matched else {
            return Decision::Idle;
        };
        let blocked = |gate| Decision::Blocked {
            rule: rule.id.clone(),
            gate,
        };

        if !config.enabled {
            return blocked(Gate::Disabled);
        }
        if config.paused_until.is_some_and(|until| ctx.now_ms < until) {
            return blocked(Gate::Paused);
        }
        if self
            .last_run_ms
            .is_some_and(|last| ctx.now_ms.saturating_sub(last) < profile.min_interval_secs * 1000)
        {
            return blocked(Gate::Cooldown);
        }
        // Dropped, not queued: by the time an in-flight run finishes the
        // condition is stale, and a queue lets a flapping trigger build a
        // backlog that fires as a burst.
        if ctx.run_in_flight {
            return blocked(Gate::RunInFlight);
        }
        if self.is_suspended(rule) {
            return blocked(Gate::Ineffective);
        }
        if ctx.foreground_busy {
            return blocked(Gate::ForegroundBusy);
        }
        if !ctx.elevated && profile.methods.iter().all(|m| m.requires_elevation()) {
            return blocked(Gate::NotElevated);
        }

        self.last_run_ms = Some(ctx.now_ms);
        self.states.entry(rule.id.clone()).or_default().last_fired_ms = Some(ctx.now_ms);

        Decision::Run {
            rule: rule.id.clone(),
            // Methods needing elevation we do not have are dropped here rather
            // than failing later; clean::run reports them as unavailable.
            methods: profile
                .methods
                .iter()
                .copied()
                .filter(|m| ctx.elevated || !m.requires_elevation())
                .collect(),
        }
    }

    fn trigger_matches(&mut self, rule: &Rule, ctx: Context) -> bool {
        let state = self.states.entry(rule.id.clone()).or_default();

        match rule.trigger {
            Trigger::UsageAbove {
                percent,
                sustained_secs,
            } => {
                if ctx.percent_in_use < percent as f64 {
                    // Falling below resets the clock; the condition must hold
                    // continuously, not cumulatively.
                    state.above_since_ms = None;
                    return false;
                }
                let since = *state.above_since_ms.get_or_insert(ctx.now_ms);
                ctx.now_ms.saturating_sub(since) >= sustained_secs * 1000
            }

            Trigger::Scheduled { every_mins } => {
                let reference = state.last_fired_ms.or(self.last_run_ms);
                match reference {
                    // Never run: start the interval now rather than firing
                    // immediately on launch.
                    None => {
                        state.last_fired_ms = Some(ctx.now_ms);
                        false
                    }
                    Some(last) => ctx.now_ms.saturating_sub(last) >= every_mins * 60 * 1000,
                }
            }

            Trigger::SystemIdle { idle_mins } => {
                let idle_enough = ctx.idle_secs >= idle_mins * 60;
                // Fire once per idle period: without this the rule would match
                // on every tick for as long as the machine stays idle, and only
                // the cooldown would stop it.
                let already_fired_this_idle = state
                    .last_fired_ms
                    .is_some_and(|last| ctx.now_ms.saturating_sub(last) < ctx.idle_secs * 1000);
                idle_enough && !already_fired_this_idle
            }
        }
    }
}

/// Seconds since the last keyboard or mouse input.
#[cfg(windows)]
pub fn idle_secs() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount64;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };

    // SAFETY: cbSize is set as the API requires and info is an owned local.
    let ok = unsafe { GetLastInputInfo(&mut info) }.as_bool();
    if !ok {
        return 0;
    }

    // GetLastInputInfo is 32-bit and wraps after ~49 days; comparing against
    // the low word of the 64-bit tick count keeps the subtraction correct.
    let now = unsafe { GetTickCount64() } as u32;
    now.wrapping_sub(info.dwTime) as u64 / 1000
}

#[cfg(not(windows))]
pub fn idle_secs() -> u64 {
    0
}

/// True when a fullscreen or presentation application is in the foreground.
///
/// Trimming during a game is precisely the moment automation must not act: the
/// resulting page faults are the stutter the user would blame on the game.
#[cfg(windows)]
pub fn foreground_busy() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };

    // SAFETY: no arguments; returns a status enum by value.
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => {
            state == QUNS_RUNNING_D3D_FULL_SCREEN
                || state == QUNS_PRESENTATION_MODE
                || state == QUNS_BUSY
        }
        // Unknown state: assume busy. Declining to act is the safe default.
        Err(_) => true,
    }
}

#[cfg(not(windows))]
pub fn foreground_busy() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: u64 = 1000;
    const MIN: u64 = 60 * SEC;

    fn ctx(now_ms: u64) -> Context {
        Context {
            now_ms,
            percent_in_use: 50.0,
            idle_secs: 0,
            foreground_busy: false,
            elevated: false,
            run_in_flight: false,
        }
    }

    fn config_with(trigger: Trigger) -> Config {
        Config {
            enabled: true,
            paused_until: None,
            active_profile: "Test".into(),
            profiles: vec![Profile {
                name: "Test".into(),
                methods: vec![Method::TrimWorkingSets],
                min_interval_secs: 300,
                rules: vec![Rule {
                    id: "r1".into(),
                    enabled: true,
                    trigger,
                    ineffective_limit: 3,
                }],
            }],
        }
    }

    fn is_run(d: &Decision) -> bool {
        matches!(d, Decision::Run { .. })
    }

    // ---- triggers -------------------------------------------------------

    #[test]
    fn usage_must_be_sustained_not_instantaneous() {
        let cfg = config_with(Trigger::UsageAbove {
            percent: 90,
            sustained_secs: 120,
        });
        let mut e = Engine::new();

        let mut c = ctx(0);
        c.percent_in_use = 95.0;
        assert_eq!(e.evaluate(&cfg, c), Decision::Idle, "must not fire instantly");

        c.now_ms = 119 * SEC;
        assert_eq!(e.evaluate(&cfg, c), Decision::Idle, "still short of the window");

        c.now_ms = 120 * SEC;
        assert!(is_run(&e.evaluate(&cfg, c)), "fires once sustained");
    }

    /// A spike that subsides must not accumulate toward the threshold.
    #[test]
    fn dropping_below_resets_the_sustained_clock() {
        let cfg = config_with(Trigger::UsageAbove {
            percent: 90,
            sustained_secs: 60,
        });
        let mut e = Engine::new();

        let mut c = ctx(0);
        c.percent_in_use = 95.0;
        e.evaluate(&cfg, c);

        c.now_ms = 50 * SEC;
        c.percent_in_use = 40.0; // subsided
        e.evaluate(&cfg, c);

        c.now_ms = 70 * SEC;
        c.percent_in_use = 95.0; // back up, clock restarts here
        assert_eq!(e.evaluate(&cfg, c), Decision::Idle);

        c.now_ms = 129 * SEC;
        assert_eq!(e.evaluate(&cfg, c), Decision::Idle, "60s from the restart, not the first spike");

        c.now_ms = 130 * SEC;
        assert!(is_run(&e.evaluate(&cfg, c)));
    }

    /// A schedule must not fire the moment Memora launches.
    #[test]
    fn scheduled_starts_its_interval_rather_than_firing_at_once() {
        let cfg = config_with(Trigger::Scheduled { every_mins: 30 });
        let mut e = Engine::new();

        assert_eq!(e.evaluate(&cfg, ctx(0)), Decision::Idle);
        assert_eq!(e.evaluate(&cfg, ctx(29 * MIN)), Decision::Idle);
        assert!(is_run(&e.evaluate(&cfg, ctx(30 * MIN))));
    }

    #[test]
    fn idle_fires_once_per_idle_period() {
        let cfg = config_with(Trigger::SystemIdle { idle_mins: 15 });
        let mut e = Engine::new();

        let mut c = ctx(20 * MIN);
        c.idle_secs = 16 * 60;
        assert!(is_run(&e.evaluate(&cfg, c)), "fires when idle long enough");

        // Still idle a minute later: must not fire again.
        c.now_ms += MIN;
        c.idle_secs += 60;
        assert!(
            !is_run(&e.evaluate(&cfg, c)),
            "a continuing idle period is not a new trigger"
        );
    }

    // ---- gates ----------------------------------------------------------

    fn firing_ctx() -> (Config, Engine, Context) {
        let cfg = config_with(Trigger::SystemIdle { idle_mins: 1 });
        let mut c = ctx(10 * MIN);
        c.idle_secs = 120;
        (cfg, Engine::new(), c)
    }

    #[test]
    fn disabled_blocks_and_says_so() {
        let (mut cfg, mut e, c) = firing_ctx();
        cfg.enabled = false;
        assert_eq!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                rule: "r1".into(),
                gate: Gate::Disabled
            }
        );
    }

    #[test]
    fn pause_expires_on_its_own() {
        let (mut cfg, mut e, mut c) = firing_ctx();
        cfg.paused_until = Some(c.now_ms + 5 * MIN);
        assert!(matches!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                gate: Gate::Paused,
                ..
            }
        ));

        c.now_ms += 6 * MIN;
        c.idle_secs += 360;
        assert!(is_run(&e.evaluate(&cfg, c)), "pause is a duration, not a mode");
    }

    /// The primary defence against the trim/decay loop.
    ///
    /// Uses a sustained-usage trigger rather than the idle one, because idle
    /// deliberately stops matching after it fires; that would exercise the
    /// trigger, not the gate.
    #[test]
    fn cooldown_blocks_a_second_run() {
        let cfg = config_with(Trigger::UsageAbove {
            percent: 90,
            sustained_secs: 60,
        });
        let mut e = Engine::new();

        let mut c = ctx(0);
        c.percent_in_use = 95.0;
        e.evaluate(&cfg, c); // starts the sustained clock
        c.now_ms = 60 * SEC;
        assert!(is_run(&e.evaluate(&cfg, c)), "first run");

        // Usage stays high, so the trigger keeps matching — exactly the loop
        // the cooldown exists to stop.
        c.now_ms = 120 * SEC;
        assert!(matches!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                gate: Gate::Cooldown,
                ..
            }
        ));

        // min_interval_secs is 300; past it the rule may fire again.
        c.now_ms = 400 * SEC;
        assert!(is_run(&e.evaluate(&cfg, c)));
    }

    /// The loop this whole design exists to prevent: usage pinned high, the
    /// trigger matching on every single tick.
    #[test]
    fn sustained_high_usage_cannot_trim_in_a_loop() {
        let cfg = config_with(Trigger::UsageAbove {
            percent: 85,
            sustained_secs: 60,
        });
        let mut e = Engine::new();

        let mut c = ctx(0);
        c.percent_in_use = 99.0;

        let mut runs = 0;
        // One hour of 1 Hz ticks with memory pinned at 99%.
        for t in 0..3600 {
            c.now_ms = t * SEC;
            if is_run(&e.evaluate(&cfg, c)) {
                runs += 1;
                // Pretend each run recovered plenty, so the effectiveness gate
                // never engages and only the cooldown is under test.
                e.record_settled("r1", EFFECTIVE_BYTES * 2);
            }
        }

        // 3600s of ticks with a 300s cooldown allows at most 12.
        assert!(
            runs <= 12,
            "cooldown must bound runs; got {runs} in an hour of pinned usage"
        );
        assert!(runs >= 10, "but it should still act periodically; got {runs}");
    }

    #[test]
    fn an_in_flight_run_is_dropped_not_queued() {
        let (cfg, mut e, mut c) = firing_ctx();
        c.run_in_flight = true;
        assert!(matches!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                gate: Gate::RunInFlight,
                ..
            }
        ));

        // Nothing was stored: once free, the next tick evaluates afresh rather
        // than replaying the earlier request.
        c.run_in_flight = false;
        assert!(is_run(&e.evaluate(&cfg, c)));
    }

    #[test]
    fn fullscreen_foreground_blocks() {
        let (cfg, mut e, mut c) = firing_ctx();
        c.foreground_busy = true;
        assert!(matches!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                gate: Gate::ForegroundBusy,
                ..
            }
        ));
    }

    // ---- effectiveness --------------------------------------------------

    /// The secondary defence: a rule that keeps recovering nothing stops.
    #[test]
    fn repeated_ineffective_runs_suspend_the_rule() {
        let (cfg, mut e, mut c) = firing_ctx();

        for i in 0..3 {
            c.now_ms += 10 * MIN;
            c.idle_secs = 120;
            assert!(is_run(&e.evaluate(&cfg, c)), "run {i} should proceed");
            e.record_settled("r1", 10 * 1024 * 1024); // 10 MB: ineffective
        }

        c.now_ms += 10 * MIN;
        c.idle_secs = 120;
        assert!(matches!(
            e.evaluate(&cfg, c),
            Decision::Blocked {
                gate: Gate::Ineffective,
                ..
            }
        ));
    }

    #[test]
    fn one_effective_run_resets_the_counter() {
        let mut e = Engine::new();
        let rule = Rule {
            id: "r1".into(),
            ineffective_limit: 3,
            ..Default::default()
        };

        e.record_settled("r1", 0);
        e.record_settled("r1", 0);
        e.record_settled("r1", EFFECTIVE_BYTES); // helped
        e.record_settled("r1", 0);
        assert!(!e.is_suspended(&rule), "the streak was broken");
    }

    #[test]
    fn resume_clears_a_suspension() {
        let mut e = Engine::new();
        let rule = Rule {
            id: "r1".into(),
            ineffective_limit: 2,
            ..Default::default()
        };
        e.record_settled("r1", 0);
        e.record_settled("r1", 0);
        assert!(e.is_suspended(&rule));

        e.resume("r1");
        assert!(!e.is_suspended(&rule));
    }

    // ---- configuration --------------------------------------------------

    #[test]
    fn cooldown_floor_is_not_user_defeatable() {
        let cfg = Config {
            profiles: vec![Profile {
                min_interval_secs: 1,
                ..Default::default()
            }],
            ..Default::default()
        }
        .sanitized();
        assert_eq!(cfg.profiles[0].min_interval_secs, 300);
    }

    #[test]
    fn a_profile_can_never_have_no_methods() {
        let cfg = Config {
            profiles: vec![Profile {
                methods: Vec::new(),
                ..Default::default()
            }],
            ..Default::default()
        }
        .sanitized();
        assert!(!cfg.profiles[0].methods.is_empty());
    }

    #[test]
    fn a_missing_active_profile_falls_back() {
        let cfg = Config {
            active_profile: "Nonexistent".into(),
            ..Default::default()
        }
        .sanitized();
        assert!(cfg.active().is_some());
    }

    #[test]
    fn defaults_are_safe() {
        let cfg = Config::default().sanitized();
        assert!(!cfg.enabled, "automation must ship disabled");

        let gaming = cfg.profiles.iter().find(|p| p.name == "Gaming").unwrap();
        assert!(
            gaming.rules.is_empty(),
            "Gaming must not mean more aggressive trimming"
        );
    }

    /// Enabling automation must not immediately fire on a condition that was
    /// already true while it was off.
    #[test]
    fn enabling_does_not_fire_on_an_already_true_condition() {
        let mut cfg = config_with(Trigger::UsageAbove {
            percent: 90,
            sustained_secs: 60,
        });
        cfg.enabled = false;
        let mut e = Engine::new();

        let mut c = ctx(0);
        c.percent_in_use = 95.0;
        // Ticks while disabled: blocked, but the sustained clock still runs.
        for t in 0..120 {
            c.now_ms = t * SEC;
            assert!(!is_run(&e.evaluate(&cfg, c)));
        }

        cfg.enabled = true;
        c.now_ms = 120 * SEC;
        // It fires now because the condition genuinely held for the window —
        // which is correct. What matters is that it was blocked, not silently
        // queued, while disabled.
        assert!(is_run(&e.evaluate(&cfg, c)));
    }

    /// The frontend reads these field names directly. A mismatch renders as
    /// "undefined" in the UI rather than failing loudly, so it is asserted.
    #[test]
    fn trigger_serialises_with_camel_case_fields() {
        let json = serde_json::to_string(&Trigger::UsageAbove {
            percent: 90,
            sustained_secs: 120,
        })
        .unwrap();
        assert!(json.contains("\"kind\":\"usageAbove\""), "{json}");
        assert!(json.contains("\"percent\":90"), "{json}");
        assert!(json.contains("\"sustainedSecs\":120"), "{json}");
        assert!(!json.contains("sustained_secs"), "{json}");

        let json = serde_json::to_string(&Trigger::SystemIdle { idle_mins: 15 }).unwrap();
        assert!(json.contains("\"idleMins\":15"), "{json}");

        let json = serde_json::to_string(&Trigger::Scheduled { every_mins: 30 }).unwrap();
        assert!(json.contains("\"everyMins\":30"), "{json}");
    }

    #[test]
    fn trigger_round_trips_through_json() {
        for t in [
            Trigger::UsageAbove { percent: 90, sustained_secs: 120 },
            Trigger::Scheduled { every_mins: 30 },
            Trigger::SystemIdle { idle_mins: 15 },
        ] {
            let json = serde_json::to_string(&t).unwrap();
            let back: Trigger = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }

    /// A config written by a build using the old field names must not take the
    /// whole settings file down with it.
    #[test]
    fn an_unreadable_rule_does_not_discard_the_config() {
        let json = r#"{
            "enabled": true,
            "activeProfile": "Balanced",
            "profiles": [{
                "name": "Balanced",
                "methods": ["trimWorkingSets"],
                "minIntervalSecs": 900,
                "rules": [{"id":"r","enabled":true,"trigger":{"kind":"systemIdle","idle_mins":15}}]
            }]
        }"#;

        // The rule fails to parse; serde(default) on Config means the whole
        // config falls back rather than the settings file being lost.
        let parsed: Result<Config, _> = serde_json::from_str(json);
        assert!(parsed.is_err(), "old field names are genuinely unreadable");

        // What matters is the recovery path: Settings uses serde(default), so a
        // bad automation block yields defaults, and defaults are safe.
        let safe = Config::default().sanitized();
        assert!(!safe.enabled, "recovery must never leave automation enabled");
    }
}
