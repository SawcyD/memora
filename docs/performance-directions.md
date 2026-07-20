# Beyond memory: what would actually make a PC faster

Status: analysis and proposal. Nothing here is implemented, and some of it
should not be.

Memora is a memory manager. This document asks what else could genuinely help
a Windows PC, ranked by real effect, and is deliberately blunt about which
popular "optimizations" do nothing or cause harm. The category Memora sits in
is full of software that ships the harmful ones because they demo well.

The test applied throughout: **can the effect be measured, and does it survive
measurement 30 seconds later?** That is the same standard the Cleaner already
holds itself to, and most PC-tuning features fail it.

---

## Tier 1 — genuinely effective, and a good fit

### 1. Startup program management

The single largest lever on how a Windows PC *feels*. A machine with 25 startup
entries takes minutes to become usable after login, and every one of those
processes holds memory for the rest of the session.

Unlike trimming, the win is permanent and compounding: an app that never starts
never needs its working set trimmed.

- **Measurable.** Windows records per-app startup impact in
  `HKCU\...\Explorer\StartupApproved` and the Task Manager startup impact data,
  and `GetProcessTimes` on early processes gives real boot-time CPU cost.
- **Fits the architecture.** Process enumeration, registry access, elevation
  handling and the settings store already exist.
- **Reversible.** Disabling is a registry flag, not a deletion — the same
  mechanism Task Manager uses, so users can undo it anywhere.
- **Honest framing available.** "This app costs ~4s of startup and 180 MB
  resident" is a fact, not a scare.

Strongest candidate by a wide margin.

### 2. Standby list management done properly

Already half-built. `PurgeStandbyList` exists but is a blunt instrument that
throws away all cache.

The genuinely useful version is narrower: purge only the **low-priority**
standby list (`MemoryPurgeLowPriorityStandbyList`, command 5, already adjacent
to the code in `clean.rs`). That reclaims cache Windows itself ranked as least
valuable, which is close to free, instead of discarding everything including
the cache making the machine fast.

Small change, strictly better default, same elevation requirement.

### 3. Storage pressure

Windows degrades sharply below roughly 10% free space: no room to defragment
metadata, restore points crowded out, and the pagefile cannot grow. On a nearly
full disk this dominates everything memory-related.

- Report free space and flag the threshold.
- Surface the *existing* Storage Sense / Disk Cleanup rather than reimplementing
  deletion. Memora should not be in the business of deleting user files, and
  the built-in tools are safer and already understood.

Read-and-recommend, not act. That keeps a genuinely destructive capability out
of the product.

---

## Tier 2 — real but narrow

### 4. Power plan awareness

On laptops, "Balanced" with aggressive processor parking measurably reduces
sustained throughput, and many machines ship on Power Saver without the user
knowing. Detecting and *reporting* this is useful.

Changing it is a system setting, so it should be a link to the Windows control,
not a switch inside Memora.

### 5. Memory leak detection

The `LeakSuspected` trigger is already specified in `docs/automation.md` as
notify-only. Worth building: a process whose working set climbs monotonically
for an hour is real information the user cannot easily get elsewhere, and
identifying it is far more valuable than trimming it repeatedly.

The restraint matters — trimming a leaking process masks the symptom while
commit charge keeps climbing.

### 6. Pagefile configuration reporting

A disabled or tiny pagefile causes commit-limit failures that look like random
application crashes. Reporting `commit_total` approaching `commit_limit` —
values Memora already reads every second — would explain a class of failure
users normally never diagnose.

Reporting only. Automatic pagefile resizing is how tuning utilities corrupt
systems.

---

## Tier 3 — do not build these

Listed because they are the standard feature set of this software category, and
their absence should be deliberate.

| Feature | Why not |
|---|---|
| **Registry cleaning** | No measurable performance effect on any modern Windows version. The registry is a database with indexed lookups; removing 400 orphaned keys from ~2 million changes nothing. Meanwhile a bad deletion breaks an application permanently. Pure downside |
| **Bulk service disabling** | The classic "services to disable" lists break Windows Update, printing, search and audio in ways users never connect back to the tool. Windows starts services on demand already |
| **Defragmenting SSDs** | Writes without benefit, consuming endurance. Windows already handles SSDs correctly with TRIM/retrim |
| **"RAM booster" widgets** | Exactly the trim-and-decay loop this codebase spent the whole design avoiding. Building a desktop widget that trims on a timer would undo the point |
| **CPU priority boosting** | The scheduler is better at this. Pinning an app to high priority typically makes the *system* less responsive, which the user experiences as the machine getting worse |
| **Network "optimization"** | TCP autotuning has been correct by default for over a decade. The registry tweaks circulating online are cargo cult |
| **Deleting `%TEMP%` aggressively** | Modest space win, real risk: applications keep live state there. Storage Sense already does this with the right exclusions |
| **Prefetch/Superfetch disabling** | SysMain exists to make things faster. Disabling it is a measurable regression on HDDs and neutral at best on SSDs |

The common pattern: they produce a satisfying number ("1,284 issues fixed") and
no measurable improvement. Memora's results page deliberately reports a figure
that is often small and sometimes negative, which is the opposite instinct and
the correct one.

---

## Scope question for the product

The spec in `CLAUDE.md` defines Memora as a **memory manager**, and the
navigation reflects that. Tier 1 items 1 and 3 are not memory features.

Two coherent options:

1. **Stay a memory tool.** Build items 2, 5 and 6, which are all genuinely
   memory work, and reject the rest. Navigation is unchanged. This keeps the
   product's claim narrow and defensible.
2. **Become a system tool.** Add a Startup page and a Storage row on Home, and
   update the spec to match. Bigger product, but startup management is where
   the real user-visible win is, and the honesty framing Memora has built would
   apply well to it.

Recommendation: **option 1 first, then item 1 of Tier 1 as a deliberate
expansion.** Standby-list refinement, leak detection and commit-limit reporting
are all small, all in-scope, and all use infrastructure that exists. Startup
management deserves its own decision rather than arriving as scope creep.

---

## Suggested order

1. Low-priority standby purge — small change to `clean.rs`, strictly better
   default than the current all-or-nothing purge.
2. Commit-limit warning on Home — data is already sampled every second.
3. Leak detection — specified already, notify-only.
4. *Decision point:* startup management, and whether Memora's scope grows.
