"""Samples a browser's processes from outside, and records the Mac's state.

Only command-line tools that ship with macOS are used, as in S1-S7: `ps` for
the process tree, `footprint` for memory, `top` for CPU time, idle wakeups and
energy impact, `pmset` and `sysctl` for the conditions.
"""

import json
import os
import plistlib
import re
import subprocess
import time

CANARY_APP = "/Applications/Google Chrome Canary.app"
SAFARI_APP = "/Applications/Safari.app"
WEBKIT_HELPERS = ("com.apple.WebKit.WebContent", "com.apple.WebKit.Networking", "com.apple.WebKit.GPU")


def app_version(app):
    with open(os.path.join(app, "Contents", "Info.plist"), "rb") as f:
        info = plistlib.load(f)
    return f"{info.get('CFBundleShortVersionString')} ({info.get('CFBundleVersion')})"


def process_table():
    """{pid: (ppid, args)} for every process."""
    out = subprocess.run(["/bin/ps", "-A", "-o", "pid=,ppid=,args="], capture_output=True, text=True, check=True)
    table = {}
    for line in out.stdout.splitlines():
        parts = line.split(None, 2)
        if len(parts) >= 2 and parts[0].isdigit():
            table[int(parts[0])] = (int(parts[1]), parts[2] if len(parts) > 2 else "")
    return table


def descendants(root, table):
    """`root` and every process below it."""
    children = {}
    for pid, (ppid, _) in table.items():
        children.setdefault(ppid, []).append(pid)
    found, stack = [], [root]
    while stack:
        pid = stack.pop()
        if pid in table:
            found.append(pid)
            stack.extend(children.get(pid, ()))
    return found


def chrome_kind(args):
    """The role of a Chrome helper process, from its command line."""
    found = re.search(r"--type=([\w-]+)", args)
    if not found:
        return "other"
    kind = found.group(1)
    if kind == "renderer":
        return "extension" if "--extension-process" in args else "renderer"
    if kind == "gpu-process":
        return "gpu"
    if kind == "utility":
        sub = re.search(r"--utility-sub-type=([\w.]+)", args)
        sub = sub.group(1) if sub else "unknown"
        return {
            "network.mojom.NetworkService": "network",
            "storage.mojom.StorageService": "storage",
            "audio.mojom.AudioService": "audio",
        }.get(sub, "utility")
    return kind


def chrome_processes(browser_pid, updater=False):
    """{pid: kind} for a Chrome instance and its helpers.

    Chrome launches its updater, GoogleUpdater, as a child; it serves no page,
    so it is left out unless `updater` is set. Exited children that Chrome
    hasn't reaped yet are left out too.
    """
    table = process_table()
    kinds = {}
    for pid in descendants(browser_pid, table):
        args = table[pid][1]
        if args.startswith("<defunct>") or args.startswith("("):
            continue
        if "GoogleUpdater" in args.split(" --")[0]:
            if updater:
                kinds[pid] = "updater"
            continue
        kinds[pid] = "browser" if pid == browser_pid else chrome_kind(args)
    return kinds


def safari_kind(args):
    executable = os.path.basename(args.split(" ", 1)[0])
    if executable == "Safari":
        return "browser"
    for helper, kind in zip(WEBKIT_HELPERS, ("renderer", "network", "gpu")):
        if executable == helper:
            return kind
    return None


def safari_processes(exclude=()):
    """{pid: kind} for Safari and the WebKit helpers running, except the pids in `exclude`.

    WebKit's helpers are children of launchd, not of Safari, and other apps that
    show web content have their own. The session by hand records which helpers
    ran before Safari started and passes them as `exclude`.
    """
    found = {}
    for pid, (_, args) in process_table().items():
        kind = safari_kind(args)
        if kind and pid not in exclude:
            found[pid] = kind
    return found


def footprints(pids, scratch, errors=None):
    """({pid: (name, footprint bytes)}, total footprint with shared memory counted once).

    `footprint`'s own errors, such as a process that exited meanwhile, are
    appended to `errors` if it is given.
    """
    pids = sorted(pids)
    if not pids:
        return {}, 0
    path = os.path.join(scratch, f"footprint-{os.getpid()}-{time.monotonic_ns()}.json")
    argv = ["/usr/bin/footprint", "--json", path, "--noCategories"]
    for pid in pids:
        argv += ["-p", str(pid)]
    subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300)
    try:
        with open(path) as f:
            data = json.load(f)
    except (OSError, ValueError):
        return {}, 0
    finally:
        if os.path.exists(path):
            os.remove(path)
    if errors is not None:
        errors.extend(str(e)[:300] for e in data.get("errors", []))
    each = {p["pid"]: (p.get("name", "?"), p.get("footprint", 0)) for p in data.get("processes", [])}
    return each, data.get("total footprint", 0)


def _seconds(clock):
    """top's TIME column (MM:SS.cc, or H:MM:SS) in seconds."""
    total = 0.0
    for part in clock.rstrip("+-").split(":"):
        total = total * 60 + float(part)
    return total


def _count(value):
    value = value.rstrip("+-")
    scale = {"K": 1e3, "M": 1e6, "G": 1e9}.get(value[-1:], 1)
    return int(float(value.rstrip("KMG")) * scale)


def parse_top(text):
    """top's samples, each {pid: (cpu %, cpu time s, idle wakeups, energy impact)}."""
    samples, current = [], None
    for line in text.splitlines():
        if line.startswith("Processes:"):
            current = {}
            samples.append(current)
            continue
        parts = line.split()
        if current is not None and len(parts) == 5 and parts[0].isdigit():
            try:
                current[int(parts[0])] = (float(parts[1]), _seconds(parts[2]), _count(parts[3]), float(parts[4]))
            except ValueError:
                pass
    return samples


def sample_top(pids, seconds):
    """CPU time, idle wakeups and mean energy impact of each pid over `seconds`, sampled once a second.

    `top -c d` counts events since the previous sample, so the first sample,
    which has no interval, is left out of the sums.
    """
    argv = ["/usr/bin/top", "-l", str(seconds + 1), "-s", "1", "-c", "d", "-stats", "pid,cpu,time,idlew,power"]
    for pid in sorted(pids):
        argv += ["-pid", str(pid)]
    out = subprocess.run(argv, capture_output=True, text=True, timeout=seconds + 60).stdout
    samples = parse_top(out)
    result = {}
    for pid in pids:
        rows = [s[pid] for s in samples if pid in s]
        if len(rows) < 2:
            continue
        result[pid] = {
            "cpu_seconds": round(rows[-1][1] - rows[0][1], 3),
            "idle_wakeups": sum(r[2] for r in rows[1:]),
            "energy_impact": round(sum(r[3] for r in rows[1:]) / (len(rows) - 1), 2),
            "samples": len(rows) - 1,
        }
    return result


def conditions():
    """The Mac's state for a formal run: power, Low Power Mode, load, memory pressure."""
    battery = subprocess.run(["/usr/bin/pmset", "-g", "batt"], capture_output=True, text=True).stdout
    settings = subprocess.run(["/usr/bin/pmset", "-g"], capture_output=True, text=True).stdout
    low_power = re.search(r"lowpowermode\s+(\d)", settings)
    pressure = subprocess.run(["/usr/bin/memory_pressure", "-Q"], capture_output=True, text=True).stdout
    free = re.search(r"System-wide memory free percentage:\s*(\d+)%", pressure)
    return {
        "ac_power": "AC Power" in battery.splitlines()[0] if battery else False,
        "low_power_mode": low_power.group(1) if low_power else None,
        "load_average": subprocess.run(["/usr/sbin/sysctl", "-n", "vm.loadavg"], capture_output=True, text=True)
        .stdout.strip()
        .strip("{} "),
        "memory_free_percent": int(free.group(1)) if free else None,
    }
