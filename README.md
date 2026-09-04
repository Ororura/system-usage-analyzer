# Rust System Monitor

Synchronous terminal monitor for CPU, memory, processes and macOS battery power.

```sh
cargo build --release
./target/release/system_usage_analyzer [MILLISECONDS] [PROCESS_COUNT]
./target/release/system_usage_analyzer tree [PID] [--sort pid|cpu|memory|name] [--interval MS]
./target/release/system_usage_analyzer power [--interval MS]
```

The default dashboard keeps Application, Memory, CPU and Top processes by memory,
and adds Battery / Health. Defaults: 500 ms refresh, 10 top processes. `tree`
shows every root; an optional PID selects a branch. `power` shows only battery
information and defaults to 2 seconds. Stop with Ctrl-C. Invalid arguments return
an error with usage instructions. The original positional syntax is preserved.

Tree sorting defaults to PID ascending. CPU and memory sort descending; names
sort ascending, case-sensitive. Ties always use PID. There is no interactive
collapse or scrolling state; large trees can exceed the terminal viewport.

CPU counters are seeded before the first frame. Process refresh intervals are
bounded below by sysinfo's minimum CPU update interval. Battery polling is never
more frequent than once every 2 seconds, even with a smaller `--interval`.
Sampling runs in one loop; collectors and renderers never sleep or spawn threads.
No Tokio is required. `power` does not refresh process metrics on each tick.

## Architecture

- **Domain:** independent process IDs/statuses, an indexed process forest,
  battery states, optional measurements, signed electrical units, repository
  contracts and typed errors. No sysinfo, IOKit, terminal or shell types.
- **Application:** `GetProcessTree` builds and sorts the forest;
  `GetPowerMetrics` caches successful results and errors using supplied time;
  `SystemMonitor` selects sources for the active view.
- **Infrastructure:** one `SysinfoMetricsProvider` owns the system counters;
  the macOS power repository wraps IOPowerSources and IORegistry; the generic
  terminal renderer writes through `Write` and propagates `io::Result`.
- **Composition:** `Config` parses CLI arguments; `main` creates adapters and
  owns pacing. `src/lib.rs` exposes the components for deterministic tests.

Tree construction uses a PID hash index and an iterative linear cycle pass:
O(n) before sorting, O(n log n) worst-case including sibling/root sorting, O(n)
storage. Missing parents and self-parent references become roots. A cycle is
broken at its lowest PID without changing the reported original PPID. Duplicate
PIDs are rejected. Selected branch depths start at zero. Arena nodes store child
indices so even deep trees can be built and dropped without recursive calls.
Only nodes reachable from the selected root are rendered. A terminated/missing
selected PID displays `Process not found` and is retried on subsequent frames.

## macOS API and measurement semantics

Base data comes directly from `IOPSCopyPowerSourcesInfo`,
`IOPSCopyPowerSourcesList`, and `IOPSGetPowerSourceDescription`, selecting only
`InternalBattery`, not UPS devices or wireless accessories. Optional fields are
expected: see [Apple's IOPowerSources documentation](https://developer.apple.com/documentation/iokit/1523867-iopsgetpowersourcedescription).

`IOServiceGetMatchingServices` and `IORegistryEntryCreateCFProperties` read
`AppleSmartBattery` for cycle count, voltage, average current, physical capacity,
external connection and temperature. No shell commands, SMC calls, administrator
privileges or battery-control writes are used.

- Charge is current / maximum capacity within the same IOPS description.
- Health is `AppleRawMaxCapacity / DesignCapacity`, using physical mAh fields.
  Normalized `MaxCapacity` values are deliberately not used. Unsupported,
  malformed or missing values remain `N/A`; health can slightly exceed 100%.
- `Voltage` is mV and signed `Amperage` is mA. Watts are calculated after both
  values are divided by 1000. Positive current means charging; negative means
  discharging. The UI shows signed current and absolute **Battery power**.
- Battery power is an estimate of energy transfer at the battery terminals,
  **not total computer power or wall-socket power**. In particular, a fully charged
  Mac on AC can have zero battery current while consuming AC power.
- Temperature uses the macOS AppleSmartBattery Smart Battery format (0.1 K),
  converted to Celsius. Unrecognized/out-of-range data stays absent; no alternate
  units are guessed. See [AppleSmartBattery source](https://github.com/apple-oss-distributions/PowerManagement/blob/main/AppleSmartBatteryManager/AppleSmartBattery.cpp).
- Time estimates use IOPS minutes. Unknown sentinels remain absent. Time to empty
  is used while discharging; time to full is used while charging.
- AC connection is independent from charging state: charging can be paused.

All application `unsafe` is confined to `infrastructure/macos/ffi.rs`, for the
small IOKit FFI surface and wrapping returned CF references. Safety comments
explain Create/Copy/Get ownership. CF wrappers and IOKit RAII handles release
owned resources; dictionary values are type-checked before conversion. Raw
pointers never reach Application or Domain.

A Mac without an internal battery displays `Not available on this device`.
Other operating systems use an `Unsupported` adapter without linking Apple
frameworks. API errors and permission failures do not stop monitoring. Failed
optional enrichment leaves base measurements available. Failed base polling
replaces the previous result with an error rather than presenting stale data
as current; a new poll is attempted after the sampling interval.

## Example output

Illustrative process tree (CPU can exceed 100% for multithreaded processes;
RAM uses binary MB, consistently with the original dashboard):

```text
Process Tree
────────────────────────────────────────────────────────────────
PROCESS                                       PID      CPU          RAM
Code                                        89597     3.4%      167.8 MB
├── Code Helper                             90071     0.5%      140.6 MB
│   ├── rust-analyzer                        90210     1.2%       92.3 MB
│   └── node                                 90231     0.2%       74.1 MB
└── git                                     90311     0.0%        8.2 MB
```

Example from a live local IOKit sample (values change over time):

```text
Battery
────────────────────────────────────────────────────────────────
Charge              99%
State               Discharging
AC connected        No
Battery power       26.2 W
Time remaining      5h 27m
Voltage             12.29 V
Current             -2.131 A

Health
────────────────────────────────────────────────────────────────
Maximum capacity    83%
Cycle count         418
Temperature         30.2 °C
```

## Validation

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
# Explicit macOS-only live API smoke test; needs no particular battery state:
cargo test live_iokit_snapshot -- --ignored --nocapture
```

Tests cover tree structure, missing/self parents, cycles, PID duplicates,
deterministic sorting, branch selection and a 50,000-node chain; unit conversion,
signed power, state mapping, optional/invalid data and capacity scales; fake
repositories, cache timing and recovery; CLI parsing, output failures, Unicode,
terminal-control sanitization and renderer placeholders. macOS-only fixture tests
exercise CF type checks and partial registry data without a real battery.

The implementation was verified on aarch64 macOS. Other-OS compilation has not
been run locally because only the aarch64-apple-darwin Rust target is installed.
