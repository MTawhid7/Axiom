# S16 Results: Lifetimes of Shared Memory and Port Rights, and Efficiency Counters

| | |
|---|---|
| **Status** | **Complete** (2026-10-22), from one formal run of three runs, with a replication. **S16-Q1:** when a renderer dies, killed or exiting, **the compositor that maps its images isn't charged for them, but the memory isn't freed:** the 64 MiB stay resident in the objects the compositor maps, or compressed, **charged to no process,** until the compositor unmaps them. The compositor is charged only for pages its CPU touches, before or after the death, so a CPU read charges both processes while the renderer lives. A dead GPU process's tiles, held by the browser as layer contents, aren't charged to the browser either. **S16-Q2:** a receiver that drops messages without destroying them keeps every right they carry: 1,000 such messages added 1,000 entries to its port space. A zero-timeout send to a full queue hands the message back with its rights and an extra reference to the destination, which leak unless the sender destroys it. Messages still queued when their port is destroyed release their rights. `IOSurfaceLookupFromMachPort` refuses names that aren't surfaces. A surface's creator can attach values its receiver reads, and the receiver can remove them. **S16-Q3:** rusage's counters are filled in for unprivileged and sandboxed children and readable by their parent at exit; **instructions retired varied 0.03–0.09% from run to run on fixed work, idle or loaded, against 2.9–12% for wall time,** so they can catch regressions at every checkpoint |
| **Question** | When a process that shared memory dies, what happens to that memory: a renderer's memory entries, which a compositor samples in place, and the GPU process's tiles, which the browser shows (S16-Q1)? What happens to port rights in messages a receiver drops, in sends that fail, and in messages queued when their port is destroyed; and what does `IOSurfaceLookupFromMachPort` do with names that aren't surfaces (S16-Q2)? Are rusage's per-process counters (instructions, energy, disk writes, peak footprints) filled in for unprivileged and sandboxed children, and steady enough across runs to serve as low-noise efficiency metrics (S16-Q3)? |
| **Feeds** | The architecture review's [Round 4](../review/round-4/02-verification.md): its checks of Round 3's claims [RV3-02](../review/round-4/02-verification.md#rv3-02-port-table-exhaustion) and [RV3-03](../review/round-4/02-verification.md#rv3-03-ghost-memory), and [the efficiency audit](../review/round-4/04-efficiency.md); [PL8](../architecture/rendering-pipeline.md), [GR12](../architecture/graphics.md), GR13, [IP5](../architecture/ipc.md), IP16, [PF4](../performance.md), [TS13](../testing.md) |
| **Plan** | The owner's authorization in plan mode (2026-10-22), choosing "S16, all three questions" from the questions as the review put them: Q1, "what happens to memory when a process that shared it dies: renderer→compositor (Gemini's 'ghost memory'), and GPU process→browser (GR13's recovery), plus the release path"; Q2, "port rights in dropped messages, failed sends and wrong-type handles (Gemini's port exhaustion)"; Q3, "whether rusage v6's per-process instruction, energy and disk-write counters work without privileges and are steady enough for per-checkpoint efficiency regression checks" |
| **Code** | `spikes/s16/lifetime` (`unsafe` only in `src/platform/`, under invariants L1–L10); `tools/s16-measure.sh` |
| **Last updated** | 2026-10-22 |

**Evidence labels** follow the [research conventions](README.md): **measured** facts come from the runs in §2; **observed** facts were read on this Mac, naming the file or command; **sourced** facts link their source; **inference** is marked.

## Summary

- **S16-Q1 (a), a renderer's images** (measured; §3): a sandboxed renderer shared 16 images of 1,024 × 1,024 pixels (64 MiB) as memory entries; the compositor mapped them, sampled them with Metal without a copy, as S10-Q2's path does, and drew each once.
  - **While the renderer lived,** the 64 MiB were charged to it (65.7 MiB), not to the compositor (16.3 MiB in the browser, 13.8 MiB in a GPU process).
  - **When it died,** killed or told to exit, **the compositor's charge didn't rise:** it fell, to 6.0 and 5.5 MiB five seconds later, as Metal released its own buffers. The objects it maps kept the 64 MiB resident and dirty, their reference counts falling from 3 to 2, and within five seconds the system often compressed them (in 14 of the 24 runs, counting both placements and both ways of dying, with and without the CPU read). **Charged to no process,** they stayed until the compositor unmapped them.
  - **A CPU read is what charges the compositor:** reading one byte of each page charged it the 64 MiB, before the death (both processes were then charged for the same pages) or after it (its charge rose from 5.5 to 69.5 MiB).
- **S16-Q1 (b), the GPU process's tiles** (measured; §3.3): the browser held 32 tiles of 256 KiB as layer contents. When the GPU process died, the browser's charge didn't rise (5.6 MiB, then 3.3 MiB), and the surfaces stayed valid.
- **S16-Q2, port rights** (measured; §4):
  - **dropped without `mach_msg_destroy`:** 1,000 messages from a sandboxed sender, each carrying its own right, added exactly 1,000 entries to the receiver's port space, and the sender never heard that the rights were released. Destroyed, they added none, and the sender got 1,000 no-senders notifications;
  - **a zero-timeout send to a full queue** (`MACH_SEND_TIMED_OUT`) came back with the moved right in the sender's space under its own name, and **with one more reference to the destination,** as XNU's pseudo-receive returns the header's ports too (§4.2). Leaked, both references stayed; destroyed, both went;
  - **five messages queued in a port when it was destroyed** released all five rights;
  - **`IOSurfaceLookupFromMachPort`** found no surface for a plain port, the null name, `MACH_PORT_DEAD` or a dead name, and consumed none of them; **the values a surface's creator attaches** (64 KiB under a key, and 4 KiB as an ICC profile) were readable by the browser, which could remove them.
- **XNU's limit** (sourced, macOS 26's kernel source): a task's port table holds at most 305,834 entries on the kernels Macs run, and a task that reaches it is ended by a fatal `EXC_RESOURCE` (§4.3).
- **S16-Q3, counters** (measured; §5): instructions, cycles, energy in nanojoules, disk bytes written, logical writes and peak footprints are all filled in, for sandboxed children too, and the parent reads them after the child exits and before it reaps it. **On fixed work, instructions retired varied 0.03–0.15% from run to run, idle or with four spinning processes, while wall time varied 1.1–37% and rose 24–137% under load.** Disk bytes written were exact. Energy varied 0.4–2.4% idle and up to 50% loaded. `ri_user_time` is in ticks of the Mach clock, not nanoseconds.

## 1. What was built

`spikes/s16/lifetime`, one binary, `s16`, which plays the browser and launches its helpers from its own executable, as S10 does:

| Mode | What it does |
|---|---|
| `memory` | S16-Q1 (a). Launches a renderer under S10's `renderer.sb`, and, with `--compositor gpu`, a GPU process under S10's `gpu.sb`. The renderer writes 16 images into memory it allocates and shares each as a memory entry, sent to the compositor; the compositor (the browser itself, or the GPU process) maps each read-only, makes a buffer over it without a copy and a linear texture over that, and draws each image once at full size, so the GPU reads every page. With `--cpu-touch` it also reads one byte of every page from the CPU. Then the renderer is killed (`--end kill`, SIGKILL) or told to exit (`--end exit`), the compositor reads every page from the CPU once, and finally unmaps everything |
| `surfaces` | S16-Q1 (b). Launches a GPU process under `gpu.sb`, which makes 32 tiles and one more, clears them with Metal, attaches a 64 KiB value and a 4 KiB "ICC profile" to the extra one, and sends the browser a send right to each. The browser looks each up, deletes its right at once (GR12), shows the tiles as the contents of a layer tree not attached to a window, reads the extra surface's values, removes them, and then the GPU process is killed or told to exit |
| `rights` | S16-Q2, §4's cases |
| `counters` | S16-Q3. Runs each workload ten times, each in a fresh child, waits for it to exit without reaping it (`waitid` with `WNOWAIT`), reads its `rusage_info_v6`, then reaps it with `wait4`; with `--load N`, N processes spin meanwhile |
| `helper`, `work` | The helpers, and S16-Q3's built-in workloads: `cpu` (sorting a fixed pseudo-random array of 2 million values, five times), `alloc` (filling 128 MiB, twice), `disk` (writing 32 MiB and syncing it), `spin` |

**Modules.** `src/platform/` holds every foreign call, under invariants L1–L10 in its `mod.rs`: S10's Mach transport, extended with raw received messages that are parsed, destroyed or deliberately leaked, failed sends that return their message, queue limits, no-senders notifications and the port space's use; memory entries, with each mapped object's residency (`mach_vm_region_recurse`), this task's `TASK_VM_INFO` and the host's page counts; `proc_pid_rusage` at `RUSAGE_INFO_V6`; IOSurface, Metal and CoreAnimation. **Three structure layouts were checked by compiling against the SDK** before they were declared (observed, 2026-10-22): `task_vm_info` is 372 bytes, packed to four, with `TASK_VM_INFO_COUNT` 93; `rusage_info_v6` 464 bytes; `vm_region_submap_info_64` 84 bytes.

**Profiles** are S10's files, included unchanged, and entered as ADR-0004 D2 has them: the browser compiles each helper's profile and passes the bytecode on an inherited socket; the GPU process resolves its cache directory first, D2's one exception.

**Dependencies:** none new: S4's library and the objc2 framework crates S10 used.

## 2. Method and environment

- **Machine:** MacBook Air M1 (`MacBookAir10,1`, 8 GB), macOS 27.2 (26B118), on AC power with Low Power Mode off (`environment.txt`); rustc 1.94.1; the Command Line Tools only, SDK 27.0.
- **The formal run:** `tools/s16-measure.sh` with its defaults, 2026-10-22 11:41–11:51 (`target/s16-results/20261022-114108/`). The script built the release binary and signed a copy ad hoc with the hardened runtime (`flags=0x10002(adhoc,runtime)`), which the helpers run too. Each of three runs began after a 10 s cool-down, once the one-minute load average was below PF1's 3.0: 2.91, 2.80 and 2.75 (`loads.txt`). The counters ran at 2.75 idle and 2.40 loaded.
- **A replication,** 11:34–11:39 (`20261022-113423/`), ran before the script waited for the load, at load averages up to 4.8. Its results show the same patterns in every configuration; the tables below are the formal run's.
- **The Mac was in use:** Chrome with video, and the window server at about a third of a core (observed with `top`, 11:57).
- **Samples (S16-Q1):** each process's `ri_phys_footprint`, lifetime and interval peaks from `proc_pid_rusage`, read by the browser; the compositor's own `TASK_VM_INFO` (the GPU process reports its own on request) and the residency of each object it maps, summed; the host's pages (`host_statistics64`); and `footprint`'s report at four stages. Stages: after the helpers start; after the images are registered and drawn; 0.1 s, 1 s and 5 s after the renderer is reaped; after the CPU read; 0.1 s and 1 s after the compositor unmaps.
- **`footprint` needs root for `--unmapped`** (observed: "footprint: Must run as root."), so it wasn't used; for several processes the tool works without root.

## 3. S16-Q1: memory after the process that shared it dies

### 3.1 A renderer's images, sampled in place

The compositor's charge (`ri_phys_footprint`, MiB), median of three runs with the range where the runs differed (`*-processes.tsv`):

| Compositor | The renderer | CPU read before the death | Start | Registered | Dead +0.1 s | Dead +5 s | CPU read after the death | Released +1 s |
|---|---|---|---|---|---|---|---|---|
| Browser | Killed | No | 6.5 | 16.3 | 12.4 (12.4–16.4) | 6.1 (6.0–10.2) | 70.1 (70.1–74.3) | 6.0 (6.0–10.2) |
| Browser | Exits | No | 6.4 | 16.3 | 16.3 (12.4–16.4) | 6.0 | 70.0 | 6.0 |
| GPU process | Killed | No | 3.8 | 13.8 | 9.7 (9.7–13.8) | 5.5 | 69.5 | 5.5 |
| GPU process | Exits | No | 3.7 | 13.7 | 13.7 | 5.5 | 69.5 | 5.5 |
| Browser | Killed | Yes | 6.5 | 80.3 | 80.4 (76.4–80.4) | 72.1 (72.1–76.4) | 72.1 (72.1–76.4) | 8.1 (8.1–12.4) |
| Browser | Exits | Yes | 6.4 | 80.3 | 76.4 (76.3–80.4) | 72.1 | 72.1 | 8.1 |
| GPU process | Killed | Yes | 3.7 | 77.8 | 73.7 (73.7–77.8) | 69.5 | 69.5 | 5.5 |
| GPU process | Exits | Yes | 3.7 | 77.7 | 73.8 (73.7–77.7) | 69.5 | 69.5 | 5.5 |

**The renderer** was charged 65.7 MiB once its images were written, in every configuration, the CPU read included (measured).

**The objects the compositor maps** (`mach_vm_region_recurse`, summed over the 16 mappings; measured):
- **while the renderer lived:** 64 MiB resident and dirty, each object with three references;
- **0.1 s after the death:** still 64 MiB resident and dirty, with two references each, in all 24 runs;
- **5 s after:** in 14 of the 24 runs, the 64 MiB had moved to the compressor (resident 0–7 MiB, swapped 56–64 MiB); in the others, still resident;
- **after the CPU read:** resident again, 64 MiB;
- **after the compositor unmapped them:** no mappings left.

**`footprint`'s view** agrees (observed, `run2/memory-gpu-kill-footprint-*.txt`): while the renderer lived, `footprint` listed the renderer at 66 MB with 64 MB of dirty "Untagged" memory "Shared with" the GPU process; after the death, the GPU process at 5,617 KB, its 23 untagged regions holding 48 KB of dirty memory; after the CPU read, 70 MB, the same regions holding 64 MB; after the release, 5,649 KB in 7 regions.

### 3.2 Why (sourced, inference)

XNU's ARM physical map keeps each task's ledgers as it enters and removes page-table entries: "a task's physical footprint is: phys_footprint = ((internal - alternate_accounting) + (internal_compressed - alternate_accounting_compressed) + iokit_mapped + purgeable_nonvolatile + purgeable_nonvolatile_compressed + page_table)", and an internal page's entry credits the `internal` ledger of the task whose table it is in ([X1], `osfmk/arm/pmap/pmap.c`; macOS 26's kernel, the latest published, 2025-10-16). **So a page is charged to every task whose CPU maps it, and to none that reaches it only through the GPU** (inference, matching §3.1). The compositor's mapping holds the object, not page-table entries; when the renderer's tables go, so does the only charge, and the pages stay resident for the mapping that remains. Gemini's Round 3 predicted that the pages would be "re-attributed" to the compositor; on this Mac they weren't, unless its CPU touched them.

### 3.3 The GPU process's tiles, held by the browser

The browser's charge (MiB), the same in each of the three runs to within 0.1 MiB (`surfaces-*-processes.tsv`):

| The GPU process | Start | Tiles shown | Dead +0.1 s | Dead +5 s | Released +1 s |
|---|---|---|---|---|---|
| Killed | 4.4 | 5.6 | 5.6 | 3.3 | 3.3 |
| Exits | 4.4 | 5.6 | 5.6 | 3.3 | 3.3 |

- **The tiles were charged to the GPU process,** 20.5 MiB with its 33 surfaces of 256 KiB, as S10 found ([S10 results](s10-results.md) §7), and not to the browser, before or after the death.
- **The surfaces stayed valid** after the GPU process died: their IDs still read, and the layer tree still held them.
- **`IOSurfaceIsInUse` read false and use counts 0** for every tile, because the layer tree isn't attached to a window, as in S10; GR12's protocol is tested with a window at M2, as planned.
- **The port space** (observed, `surfaces-*-notes.txt`): the browser held 24 entries before the commit and 57 with the 33 rights. The first lookup added about 19 entries, IOSurface's client setting itself up once, and each later lookup and deletion freed one entry, ending at 43. So a surface leaves no entry behind once its right is deleted.

## 4. S16-Q2: port rights

### 4.1 Results

The same in each of the three runs (`rights.tsv`):

| Case | What was done | What it left |
|---|---|---|
| **Dropped, one right each** | A sandboxed sender sent 1,000 messages, each moving a send right to a port of its own; the receiver dropped them **without** `mach_msg_destroy` | **+1,000 entries** in the receiver's port space (25 → 1,025); **0** no-senders notifications at the sender |
| | The same, **with** `mach_msg_destroy` | No new entries; **1,000** no-senders notifications |
| **Dropped, one right copied** | 1,000 messages, each copying one send right; dropped without destroying | One entry, with **1,000 user references**; no notification |
| | The same, destroyed | No entry left; one notification |
| **A failed send** | A zero-timeout send, moving a right, to a port whose queue (limit 1) was full | `MACH_SEND_TIMED_OUT`; the message came back with the moved right **under the same name**, and **the destination's references rose from 1 to 2.** Leaked: both stayed. Destroyed: both went |
| **Queued at destruction** | Five messages, each carrying the only send right to a port, queued in a port that was then destroyed | **5 of 5** no-senders notifications: the kernel released the rights |
| **Lookups** | `IOSurfaceLookupFromMachPort` given a plain port's send right, the null name, `MACH_PORT_DEAD` and a dead name; then a surface's own right | No surface for the first four; the surface for the last. **No reference was consumed** (1 before and after) |

### 4.2 Why the destination gains a reference (sourced)

When a send fails after its rights have been copied in, XNU "does a pseudo-copyout of the message. This is like a regular copyout, except that the ports in the header are handled as if they are in the body" (`ipc_kmsg_copyout_pseudo`), called from the send path as `mach_msg_receive_pseudo` when `ipc_kmsg_send` fails ([X1], `osfmk/ipc/ipc_kmsg.c`, `osfmk/ipc/mach_msg.c`). The SDK says the same of `voucher_mach_msg_clear`: "an unsuccessful send operation (pseudo-receive)" (observed, `mach/mach.h`). So the copied send right to the destination is back in the sender's space, and `mach_msg_destroy` on the returned message releases it with the rest.

### 4.3 How large the space can grow (sourced)

- **The table's ceiling:** `CONFIG_IPC_TABLE_ENTRIES_SIZE_MAX=7340032 # 7M == 305834 entries`, for the medium, large and extra-large configurations, and 3 MiB, 131,072 entries, for the small ones ([X1], `config/MASTER`). Which configuration this Mac's kernel uses wasn't observed; Macs aren't among the small ones (inference).
- **At the ceiling:** "Abort the process if it has hit the system-wide limit for ipc port table size", with an `EXC_RESOURCE` of type `RESOURCE_TYPE_PORTS` and flavor `FLAVOR_PORT_SPACE_FULL` ([X1], `osfmk/kern/task.c`).
- **So** a renderer that can make the browser drop messages carrying rights, without destroying them, could end the browser after about 305,000 such messages (inference from §4.1).

## 5. S16-Q3: counters

Ten runs of each workload, each in a fresh child, idle and with four spinning processes (`counters-*.tsv`, `counters-*-summary.txt`):

| Workload | Instructions, mean | Run-to-run variation: idle | Loaded | Wall time, idle | Variation | Loaded | Variation | Energy variation: idle | Loaded |
|---|---|---|---|---|---|---|---|---|---|
| `cpu` | 1,673,039,377 | 0.03% | 0.09% | 149.1 ms | 2.9% | 193.7 ms | 12.0% | 2.4% | 4.8% |
| `sandboxed-cpu`, under `renderer.sb` | 1,674,181,079 | 0.04% | 0.15% | 154.5 ms | 1.1% | 199.2 ms | 7.2% | 1.8% | 5.4% |
| `alloc` | 32,601,152 | 0.09% | 0.15% | 4.9 ms | 6.3% | 11.7 ms | 37.3% | 0.9% | 49.9% |
| `disk` | 66,372,129 | 6.47% | 0.85% | 32.0 ms | 13.8% | 34.4 ms | 26.8% | 14.3% | 22.1% |
| S3's `s3-count` | 7,820,087,785 | 0.05% | 0.07% | 575.5 ms | 2.3% | 739.3 ms | 5.0% | 0.6% | 1.4% |
| S10's `validate`, 50 iterations | 35,068,816,021 | 0.21% | 0.98% | 1,643.6 ms | 3.8% | 2,037.5 ms | 4.6% | 0.4% | 2.3% |

Variation is the coefficient of variation (the standard deviation over the mean).

- **Every counter is filled in** for unprivileged and sandboxed children: instructions, cycles, the performance cores' share (`ri_pinstructions`, `ri_pcycles`), `ri_energy_nj`, disk bytes read and written, logical writes, and the lifetime and interval peaks of the footprint. The parent reads them after the child exits and before it reaps it, through `proc_pid_rusage` (measured).
- **Instructions retired are steady:** on fixed work, 0.03–0.15% from run to run, and their means moved by 0.10–0.44% under load, while wall time rose 24–137%. A regression of 1% in instructions stands out from that noise at once, with the Mac in use (inference).
- **Fixed work matters:** S10's `validate` warms the core up for 100 ms before it measures ([S10 results](s10-results.md) §2), so how much it does depends on speed, and its instructions moved 2.7% under load. `disk`'s instructions vary with the kernel's path through the file system (6.5% idle).
- **Disk bytes written were exact,** 33,554,432 in every run, and logical writes varied 0.2%.
- **Peak footprints** were exact on fixed work idle, and within 0.8% loaded.
- **Energy** varied 0.4–2.4% idle on CPU-bound work, but load changes how the system attributes it: `s3-count`'s mean fell 8.5% under load as its time rose. So energy is a gate measurement, under PF1's conditions, rather than a regression check (inference).
- **`ri_user_time` counts ticks of the Mach clock,** not nanoseconds: it read 0.0240 of `wait4`'s user time in every run, which is 3 ÷ 125, this Mac's timebase (measured).

## 6. Implications for Axiom

Each is an inference for Round 4's findings to place ([the findings](../review/round-4/06-findings.md)):
- **A dead renderer's resources must be released by the compositor at once,** on the death notification the browser already gets (IP6, IR10): the browser's compositor thread at M1, the GPU process from M2. Until then their memory is resident, or compressed, and charged to no process, so no budget sees it (PF4, B1, B3).
- **The compositor never reads renderer memory with its CPU.** A read charges the pages to both processes while the renderer lives, and to the compositor once it dies. That covers any CPU fallback, such as a software path or a copy.
- **A received message is either parsed, with every right handed to an owner, or destroyed.** A message dropped as an expected race (IP11's `Ignore`), a rejected one and an unparsable one must be destroyed, or a renderer can grow the browser's port space until the kernel ends it. **A failed send's returned message is destroyed or kept whole,** with its names, for a retry.
- **IOSurfaces from the GPU process carry values the browser shouldn't trust.** The browser can remove them after lookup, and set what it needs from the commit's validated fields.
- **Instructions retired, disk bytes written and peak footprints can gate every checkpoint,** on fixed workloads, without PF1's conditions; wall time and energy stay with the milestone gates, under PF1.

## 7. Threats to validity

1. **One machine and one OS build:** macOS 27.2 (26B118) on an M1. XNU's source for macOS 27 isn't published (§3.2, §4.3 cite macOS 26's), so the accounting mechanism is inferred for this kernel from the measurements, which it matches.
2. **No window.** The layer tree wasn't attached to a window, so the window server never used the tiles, which is why `IOSurfaceIsInUse` read false; GR12's protocol needs M2's test with a window.
3. **The Mac was in use,** with Chrome playing video. The formal runs began below PF1's load threshold, but the load rose during them (6.4 at the end). Memory and Mach semantics don't depend on it; the counters' loaded case is that condition made worse on purpose.
4. **Compression timing depends on memory pressure:** whether the orphaned pages were compressed within 5 s varied from run to run (§3.1). That they stay charged to nobody does not.
5. **S16's "renderer" and "GPU process" are stand-ins,** with S10's profiles: the renderer allocates and shares; the GPU process maps and draws. A real compositor would hold more state, which doesn't change the accounting.
6. **The port table wasn't exhausted,** to avoid a fatal exception; its limit and what happens at it come from XNU's source (§4.3).
7. **Cool-downs of 10 s,** not PF1's 60 s: S16 measures memory, Mach semantics and counts of work, none of them timings that heat changes; its wall times and energy (§5) are reported only to show their noise against the counters', so the shorter cool-downs, if anything, widen that noise (inference).

## 8. Reproducing these results

```sh
cargo test -p s16-lifetime                       # builds; the crate has no unit tests of its own
tools/s16-measure.sh                             # every question, three runs; needs AC power
tools/s16-measure.sh --quick                     # a short pass for checking the script
target/s16/bin/s16 memory --compositor gpu --end kill --out DIR   # one configuration
target/s16/bin/s16 rights --count 1000 --out DIR
target/s16/bin/s16 counters --runs 10 --load 4 --label loaded --out DIR -- target/release/s3-count
```

| What | Where |
|---|---|
| The formal run | `target/s16-results/20261022-114108/` |
| The replication, before the load wait | `target/s16-results/20261022-113423/` |
| A quick pass, checking the script | `target/s16-results/20261022-113243/` |

## 9. Side effects

- **No crash reports:** the script compared `~/Library/Logs/DiagnosticReports` before and after each run, and found none new. SIGKILL leaves none.
- **Metal printed** `sandbox_extension_issue_file failed for … com.apple.metalfe` and `… com.apple.gpuarchiver` under `gpu.sb` (observed in each GPU process's standard error): Metal tries to extend its sandbox to two cache directories the profile doesn't name, fails, and carries on. S10's profile works without them; M2's removal tests can confirm that nothing slows.
- **Temporary files:** the `disk` workload's 32 MiB file, written and deleted under `target/`, ten times per run.
- **Metal's shader cache** gained S16's one small pipeline.

## Sources

- [X1] XNU, `xnu-12377.121.6` (committed 2025-10-16, macOS 26's kernel, the latest published), read 2026-10-22: `config/MASTER`, `osfmk/kern/task.c`, `osfmk/ipc/mach_msg.c`, `osfmk/ipc/ipc_kmsg.c`, `osfmk/arm/pmap/pmap.c` — https://github.com/apple-oss-distributions/xnu/tree/xnu-12377.121.6
- SDK 27.0 of the Command Line Tools, observed 2026-10-22: `sys/resource.h` (`RUSAGE_INFO_V6`), `mach/mach.h` (`mach_msg_destroy`), `mach/message.h`, `mach/task_info.h`, `mach/vm_statistics.h`, `mach/vm_region.h`, `mach_debug/ipc_info.h`.
- This repository: [S10 results](s10-results.md); the review's [Round 3](../review/round-3/01-gemini-review.md), whose claims §3.2 and §4 test.
