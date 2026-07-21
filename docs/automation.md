# Automation — operational design

Status: design, not implemented. This document defines how the Automation
feature behaves before any code is written.

Automation runs memory optimizations without the user asking each time. That
makes it the most dangerous feature in Memora, for a reason specific to what
optimization actually does — see [The constraint that shapes everything](#the-constraint-that-shapes-everything)
before reading the rest.

---

## The constraint that shapes everything

Trimming a working set does not free memory. It moves pages to the standby
list, where they stay resident and are faulted back in as processes resume.
`clean.rs` documents this, the Cleaner results page states it, and the report
re-measures available memory 30 seconds later precisely because the immediate
gain decays.

A naive rule — *"when usage exceeds 85%, optimize"* — therefore does this:

1. Usage crosses 85%. Trim runs. Available memory rises.
2. Processes resume and fault their pages back. Usage returns to 85%.
3. The rule fires again.

The result is a loop that generates continuous page faults and disk I/O,
making the machine slower while reporting that it is helping. This is the
failure mode that gives "memory optimizers" their reputation, and it is
reached by implementing the obvious design.

Three mechanisms in this document exist solely to prevent it, and none are
optional: **cooldowns**, **effectiveness feedback**, and **conservative
defaults**. Automation ships disabled.

---

## Architecture fit

Automation adds no new sampling, scheduling, or process infrastructure. It is
an evaluator that consumes the existing 1 Hz sample and calls the existing
runner.

```
system::memory::snapshot()          1 Hz sampler thread (exists)
        │
        ├──> emit "memory://sample" ──> window graph        (exists)
        ├──> tray::update()                                 (exists)
        └──> automation::evaluate()                         NEW
                     │
                     │ rule matched, gates passed
                     ▼
             system::clean::run()                           (exists)
                     │
                     ├──> "clean://progress" / "done" / "failed"   (exists)
                     ├──> "clean://settled"  (30s re-measure)      (exists)
                     ├──> notify_result()                          (exists)
                     └──> history::record()                        NEW
```

New modules: `system::automation` (rules, evaluator, gates) and
`system::history` (durable run records). Everything else is reuse.

### Why the existing sampler

The sampler already runs at a fixed 1 Hz regardless of tray interval, because
the graph needs a steady series. Evaluating rules on that same tick means
automation cannot disagree with what the user sees on the Home graph, and adds
no timer of its own. Rules needing coarser granularity (schedules, idle) keep
their own counters and are evaluated on the same tick.

---

## Data model

Stored in `settings.json` alongside existing settings, using the same `Store`
load/`sanitized()`/save path so malformed values are clamped rather than
trusted.

```rust
struct AutomationConfig {
    enabled: bool,              // master switch, default false
    paused_until: Option<u64>,  // epoch ms; set by "Pause automatic cleaning"
    active_profile: String,
    profiles: Vec<Profile>,
}

struct Profile {
    name: String,               // Balanced | Gaming | Development | custom
    methods: Vec<Method>,       // reuses system::clean::Method
    excluded: Vec<String>,      // process NAMES, not pids — see note below
    rules: Vec<Rule>,
    min_interval_secs: u64,     // cooldown, default 900 (15 min)
}

struct Rule {
    id: Uuid,
    enabled: bool,
    trigger: Trigger,
    /// Consecutive ineffective runs before this rule self-suspends.
    ineffective_limit: u32,     // default 3
}

enum Trigger {
    UsageAbove   { percent: u8, sustained_secs: u64 },
    Scheduled    { every_mins: u64 },
    SystemIdle   { idle_mins: u64 },
    ProcessExits { name: String },
    LeakSuspected { name: String, growth_mb: u64, over_mins: u64 },
}
```

**Exclusions must move to process names.** They currently live in React state
in `App.tsx` as a `number[]` of pids, lost on restart. Pids are reassigned
across reboots, so a persisted pid list would eventually exclude an unrelated
process. Automation cannot ship until exclusions are persisted and matched by
name. This is a prerequisite, not a nice-to-have.

---

## Triggers

| Trigger | Fires when | Default | Rationale |
|---|---|---|---|
| `UsageAbove` | usage ≥ `percent` continuously for `sustained_secs` | 90% / 120s | Sustained, never instantaneous — a momentary spike during app launch is normal and self-corrects |
| `Scheduled` | `every_mins` elapsed since last automatic run | off | Predictable; no feedback loop with usage |
| `SystemIdle` | no input for `idle_mins` (`GetLastInputInfo`) | 15 min | Safest window: the decay cost is paid while nobody is working |
| `ProcessExits` | a named process transitions present → absent | off | The genuinely useful case — reclaiming after a game or build closes |
| `LeakSuspected` | one process's working set grows ≥ `growth_mb` monotonically over `over_mins` | off, notify-only | Trimming does not fix a leak; the honest action is to tell the user, not to mask it |

`LeakSuspected` **never triggers an optimization**. It raises a notification
and a History entry. Repeatedly trimming a leaking process hides the symptom
while the leak continues to consume commit charge, which is worse for the user
than knowing.

### Trigger evaluation

Each rule holds its own small state (a counter, a last-fired timestamp, a
ring of recent samples). On each 1 Hz tick, every enabled rule in the active
profile is evaluated in declaration order. The first rule to match wins and
evaluation stops — one tick can produce at most one run request.

---

## Gates

A matched rule does not run an optimization. It produces a *run request* that
must clear every gate below, in order. Each gate that blocks records a reason;
reasons are surfaced in History rather than discarded, so "why didn't it run"
is always answerable.

| # | Gate | Blocks when | Why |
|---|---|---|---|
| 1 | Master | `enabled == false` | Automation ships off |
| 2 | Pause | `paused_until` in the future | Tray "Pause automatic cleaning" |
| 3 | Cooldown | last automatic run < `min_interval_secs` ago | Primary loop defence |
| 4 | Single-flight | an optimization is in flight | `CleanTask` already returns `"An optimization is already running"` |
| 5 | Effectiveness | rule's consecutive ineffective runs ≥ `ineffective_limit` | Secondary loop defence |
| 6 | Foreground | a fullscreen/exclusive app is foreground | Trimming mid-game causes stutter — the exact moment automation must not act |
| 7 | Elevation | rule requires privileged methods without elevation | Skipped with a stated reason, matching Cleaner behaviour |

Gate 4 is not new work. `start_optimization` already rejects concurrent runs;
automation calls the same path and treats the rejection as a blocked request.

### Effectiveness feedback (gate 5)

This is what stops the loop that a cooldown alone only slows down.

`clean://settled` already reports available memory 30 seconds after a run —
the figure that survived decay. Automation subscribes to it and classifies the
run:

- **Effective** — settled gain ≥ 200 MB. Reset the rule's ineffective counter.
- **Ineffective** — settled gain < 200 MB. Increment the counter.

After `ineffective_limit` consecutive ineffective runs (default 3), the rule
self-suspends and raises one notification:

> Memora paused a cleaning rule
> "When memory is above 90%" freed little memory on the last 3 runs and has
> been paused. Your system may simply be using the memory it has.

Self-suspension is per-rule, is surfaced in the Automation page with a
one-click resume, and resets when the user edits the rule. **A rule that is not
helping stops on its own.** The last sentence of that notification matters: high
memory usage is often correct behaviour, and the honest outcome is to say so
rather than keep trimming.

---

## Actions

A request that clears all gates performs, in order:

1. `system::clean::run(profile.methods, profile.excluded_pids, cancel, on_progress)`
   — the same call the Cleaner button makes, on a worker thread.
2. Emit the existing `clean://*` events, so an open window shows the run live
   on the Cleaner page with a working Cancel button. **Automatic runs are
   cancellable by the user exactly like manual ones.**
3. `notify_result()` if enabled — reusing the existing wording, which says the
   gain was "immediate" rather than claiming memory was freed.
4. Record to History (below), including trigger, gates passed, and the settled
   result once it arrives 30s later.

Profile switching is the other action type: `Scheduled`/`ProcessExits` triggers
may set `active_profile` instead of running an optimization (e.g. activate
Gaming when a game launches). A profile change notifies once, per the spec's
"automatic profile changes".

---

## History (new dependency)

Automation is not shippable without it. A feature that acts unattended must be
auditable, and the History nav item is currently a placeholder.

Append-only JSON-lines at `{app_config_dir}/history.jsonl`, capped by count and
age (default 500 records / 90 days), rotated on write.

```rust
struct HistoryRecord {
    at: u64,                       // epoch ms
    source: Source,                // Manual | Tray | Automation { rule_id }
    outcome: Outcome,              // Completed | Cancelled | Failed | Blocked { gate }
    methods: Vec<Method>,
    available_before: u64,
    recovered_immediate: i64,
    recovered_settled: Option<i64>, // None until the 30s measurement lands
    processes_trimmed: u32,
    processes_skipped: u32,
    errors: u32,
    duration_ms: u64,
    unavailable: Vec<String>,
}
```

`recovered_settled` is `Option` for the same reason every counter in
`MemoryDetail` is: the app may exit before the 30-second measurement, and a
missing measurement is unknown, not zero.

Blocked requests are recorded too. Without them, a user whose automation never
fires has no way to discover that a cooldown or the effectiveness gate is
responsible.

---

## Edge cases

### Failure during a run

`clean::run` already returns `Result` and per-process outcomes; inaccessible
processes are `Skipped`, not `Failed`. Automation adds:

- A failed automatic run notifies (failures notify regardless of the results
  toggle, per existing behaviour) and records `Outcome::Failed`.
- Failure does **not** disable automation. It increments a per-rule failure
  count; 3 consecutive failures self-suspend that rule with the same mechanism
  as ineffectiveness.
- A panic in the evaluator must not take down the sampler thread. The evaluator
  runs inside `catch_unwind`; a panic disables automation for the session and
  records the reason. The tray meter and graph keep working — automation is the
  least important thing on that thread.

### Incomplete or invalid configuration

Handled at the `Store` boundary, extending the existing `sanitized()` pattern:

| Condition | Behaviour |
|---|---|
| Unknown enum variant (older/newer build) | Rule loads `enabled: false`, flagged in the UI. Never coerced to a different trigger — silently changing what a rule does is worse than not running it |
| Out-of-range numeric | Clamped, as thresholds and intervals already are |
| `min_interval_secs` below floor | Clamped to 300s. The cooldown floor is not user-defeatable |
| `active_profile` names a missing profile | Falls back to the first profile; if none, automation disables and says so |
| Empty `methods` | Rule disabled — a run that does nothing would still burn a cooldown and pollute effectiveness stats |
| Corrupt `settings.json` | Existing behaviour: defaults load. Automation defaults to off, so corruption can never *enable* unattended action |
| Corrupt `history.jsonl` | Unparseable lines skipped, file kept. History is evidence; truncating it on a bad line destroys the audit trail |

### Concurrent triggers

Three distinct cases:

**Two rules match on the same tick.** Impossible by construction — evaluation
stops at the first match. Order is the user's declared priority.

**A rule matches while a run is in flight.** Gate 4 blocks it. The request is
**dropped, not queued.** Queuing is wrong here: by the time the in-flight run
finishes, the condition that matched is stale, and a queue lets a flapping
trigger accumulate a backlog that fires as a burst. The next tick re-evaluates
against current reality.

**A rule matches while the user is running a manual optimization.** Same gate,
same outcome — dropped. The user's explicit action always wins; automation
never cancels, delays, or interferes with a manual run.

**Two Memora instances.** Prevented upstream by a single-instance lock, which
Memora needs anyway — two tray meters would already be a bug. Without it, two
processes would race on `settings.json` and `history.jsonl`.

### Sleep, resume, and clock changes

`Scheduled` and `SystemIdle` use monotonic elapsed time (`Instant`), not wall
clock, so a clock change or DST shift cannot fire a rule. On resume from sleep,
missed windows are **not** replayed — at most one run is requested, after a
60-second settling delay, because resume already involves heavy paging and
trimming into it would compound the stall.

### Elevation changes mid-session

Elevation cannot change without a restart, so it is read once per run via the
existing `is_elevated()` check. Privileged methods in a rule are skipped with a
stated reason and recorded in `unavailable`, matching what the Cleaner already
does. A rule whose *only* methods require elevation is reported as blocked
rather than silently running as a no-op.

---

## User workflow compatibility

- **Nothing changes for existing users.** Automation ships disabled; a user who
  never opens the Automation page sees identical behaviour.
- **Manual control is never removed.** Automatic runs appear on the Cleaner page
  with the same progress UI and a working Cancel.
- **The tray gains** "Pause automatic cleaning" and the profile submenu already
  described in the spec. Pause is a duration (default 1 hour), not a permanent
  toggle, so a user who pauses to play a game is not silently unprotected
  forever.
- **The Cleaner page is unchanged.** Automation reuses its methods and risk
  labels rather than introducing a second set of definitions.

---

## Prerequisites

Ordered. Automation should not begin until the first three land.

1. **Persisted, name-based exclusions.** Currently in-memory pids in `App.tsx`.
   Blocking — automation acting on stale pids could trim a process the user
   meant to protect.
2. **History storage and page.** Blocking — unattended action requires an audit
   trail.
3. **Single-instance lock.** Blocking for correctness of the settings/history
   writes.
4. **Profiles.** The data model above; the tray submenu is specified but unbuilt.
5. **Idle detection** (`GetLastInputInfo`) and **foreground/fullscreen detection**
   (`SHQueryUserNotificationState`) for gate 6 and the idle trigger.
6. **Verified notification delivery.** Toasts currently do not render from a dev
   build — Windows drops them without a Start Menu shortcut backing the
   AppUserModelID. Automation leans on notifications far more than manual use
   does, since the user is not watching, so delivery must be confirmed against
   an installed build first.

---

## Open questions

- **Default profile contents.** Balanced almost certainly means "idle trigger,
  trim working sets only". Gaming is arguably "do nothing while playing, run
  once on exit" — which is a `ProcessExits` rule plus a foreground gate, not a
  more aggressive profile. Worth confirming that the aggressive-sounding name
  does not imply aggressive behaviour.
- **Effectiveness threshold.** 200 MB is a guess. It should be derived from real
  `clean://settled` data across a few machines before being fixed.
- **Should `LeakSuspected` offer to end the process?** It can identify a
  suspected leak but ending a process is destructive and the detection is
  heuristic. Current position: notify only, never offer.
