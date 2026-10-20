#!/usr/bin/env python3
"""Spike S8, the corpus census (Step 6 tracker §10.1; docs/corpus.md).

    python3 -I spikes/s8/s8.py COMMAND [options]

Commands:
  prepare        download Chromium's use-counter enums for the installed Canary
                 into target/s8/ (two files from github.com/chromium/chromium)
  check          the Mac's conditions, the browsers' versions, and one run of
                 the census on a local test page, served on 127.0.0.1
  robots         the robots.txt verdict for each automated page (corpus CO3)
  auto           the automated census of the pages the corpus permits (CO4):
                 Chrome Canary, fresh profiles, --runs runs after cool-downs
  signin         open Chrome Canary's dedicated profile (scratch/s8/), for the
                 owner to sign in by hand; nothing is measured
  hand           the census by hand (CO6): the owner drives Chrome Canary or
                 Safari, and only the browser's processes are sampled
  safari-status  whether Safari accepts WebDriver sessions (remote automation),
                 without changing any setting
  summary DIR    medians and spreads of a results directory

Results go to target/s8-results/<timestamp>/, which git ignores. The census
keeps aggregate figures only (CO13): no page content, cookies or URLs beyond
the corpus pages themselves.
"""

import argparse
import http.client
import http.server
import json
import os
import socket
import statistics
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request

sys.dont_write_bytecode = True  # no __pycache__ beside the spike's sources
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import census  # noqa: E402
import robots  # noqa: E402
import system  # noqa: E402
import usecounters  # noqa: E402
from corpus import BY_IDENT, PAGES, full_urls  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TARGET = os.path.join(ROOT, "target", "s8")
RESULTS = os.path.join(ROOT, "target", "s8-results")
SCRATCH = os.path.join(ROOT, "scratch", "s8")
URL_LIST = os.path.join(ROOT, "scratch", "corpus-urls.txt")
TEST_PAGE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "testpage")
AGENT = "AxiomCensus"


def canary_version():
    return system.app_version(system.CANARY_APP).split(" ")[0]


def enum_dir():
    return os.path.join(TARGET, f"chromium-{canary_version()}")


def load_names():
    names = {}
    for key, histogram in (("features", usecounters.FEATURES), ("css", usecounters.CSS_PROPERTIES),
                           ("webdx", usecounters.WEBDX)):
        path = os.path.join(enum_dir(), os.path.basename(usecounters.ENUM_FILES[histogram]))
        if not os.path.exists(path):
            sys.exit(f"{path} is missing: run `prepare` first")
        with open(path, encoding="utf-8", errors="replace") as f:
            names[key] = usecounters.parse_enum(f.read())
    return names


def environment():
    return {
        "date": time.strftime("%Y-%m-%d %H:%M:%S"),
        "macos": subprocess.run(["/usr/bin/sw_vers", "-productVersion"], capture_output=True, text=True).stdout.strip(),
        "chrome_canary": system.app_version(system.CANARY_APP),
        "safari": system.app_version(system.SAFARI_APP),
        "conditions": system.conditions(),
        "appearance": subprocess.run(["/usr/bin/defaults", "read", "-g", "AppleInterfaceStyle"], capture_output=True,
                                     text=True).stdout.strip() or "Light",
        "commit": subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        + ("" if not subprocess.run(["git", "-C", ROOT, "status", "--porcelain", "--untracked-files=no"],
                                    capture_output=True, text=True).stdout.strip() else " (uncommitted changes)"),
        "displays": [
            line.strip()
            for line in subprocess.run(["/usr/sbin/system_profiler", "SPDisplaysDataType"], capture_output=True,
                                       text=True).stdout.splitlines()
            if any(k in line for k in ("Resolution", "UI Looks like", "Main Display"))
        ],
    }


def require_conditions(allow_battery):
    state = system.conditions()
    if not allow_battery and (not state["ac_power"] or state["low_power_mode"] != "0"):
        sys.exit(f"not on AC power with Low Power Mode off ({state}); --allow-battery runs anyway")
    return state


def new_results_dir(kind):
    path = os.path.join(RESULTS, f"{time.strftime('%Y%m%d-%H%M%S')}-{kind}")
    os.makedirs(path, exist_ok=True)
    return path


def write_json(path, value):
    with open(path, "w") as f:
        json.dump(value, f, indent=1, sort_keys=True)


def cmd_prepare(args):
    version = canary_version()
    os.makedirs(enum_dir(), exist_ok=True)
    for source in usecounters.ENUM_FILES.values():
        path = os.path.join(enum_dir(), os.path.basename(source))
        if os.path.exists(path):
            print(f"{path}: present")
            continue
        url = f"https://raw.githubusercontent.com/chromium/chromium/{version}/{source}"
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read()
        with open(path, "wb") as f:
            f.write(data)
        print(f"{path}: {len(data)} bytes from {url}")


class _Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def serve_test_page():
    """Serves spikes/s8/testpage on 127.0.0.1 from a background thread; returns its URL."""
    handler = lambda *a, **k: _Quiet(*a, directory=TEST_PAGE, **k)  # noqa: E731
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{server.server_address[1]}/index.html"


def cmd_check(args):
    print(json.dumps(environment(), indent=1))
    names = load_names()
    os.makedirs(TARGET, exist_ok=True)
    with open(os.path.join(TARGET, "check-chrome.log"), "ab") as log:
        version = census.default_user_agent(log, TARGET)
        print(f"Canary over the pipe: {version.get('product')}; protocol {version.get('protocolVersion')}")
        user_agent = f"{version['userAgent']} {census.TOKEN}"
        record = census.measure("testpage", serve_test_page(), user_agent, names, TARGET, log, phase_seconds=args.seconds)
    out = new_results_dir("check")
    write_json(os.path.join(out, "testpage.json"), record)
    print(f"error: {record.get('error')}")
    print(f"features ({len(record.get('features', []))}): {', '.join(record.get('features', []))}")
    print(f"groups: {json.dumps(usecounters.groups(record.get('features', [])))}")
    print(f"web-features: {', '.join(record.get('web_features', []))}")
    print(f"histograms seen: {record.get('histograms_seen')}")
    print(f"network: {json.dumps(record.get('network'))}")
    print(f"dom: {json.dumps(record.get('dom_loaded'))}")
    print(f"timing: {json.dumps(record.get('timing'))}")
    print(f"memory: {json.dumps(record.get('memory_loaded'))}")
    print(f"without input: {json.dumps(record.get('without_input'))}")
    print(f"targets: {record.get('targets')}; interaction: {record.get('interaction')}")
    print(f"record: {out}/testpage.json")


def robots_verdicts(user_agent):
    verdicts = {}
    for page in PAGES:
        if page.automated:
            parts = urllib.parse.urlsplit(page.url)
            path = parts.path + (f"?{parts.query}" if parts.query else "")
            verdicts[page.ident] = robots.check(f"{parts.scheme}://{parts.netloc}", path, AGENT, user_agent)
    return verdicts


def cmd_robots(args):
    for ident, (allowed, reason) in robots_verdicts(f"{AGENT}/0.1 (+https://github.com/MTawhid7/Axiom)").items():
        print(f"{ident:10} {'allowed' if allowed else 'REFUSED'}: {reason}")


def cmd_auto(args):
    state = require_conditions(args.allow_battery)
    names = load_names()
    pages = [BY_IDENT[p] for p in args.pages.split(",")] if args.pages else [p for p in PAGES if p.automated]
    for page in pages:
        if not page.automated:
            sys.exit(f"{page.ident}: its site's terms or robots file don't permit automated loads (corpus CO3)")
    out = new_results_dir("auto")
    env = environment()
    env["options"] = {"runs": args.runs, "cooldown": args.cooldown, "seconds": args.seconds, "pages": [p.ident for p in pages]}
    os.makedirs(TARGET, exist_ok=True)
    with open(os.path.join(out, "chrome.log"), "ab") as log:
        version = census.default_user_agent(log, TARGET)
        user_agent = f"{version['userAgent']} {census.TOKEN}"
        env["chrome_product"] = version.get("product")
        verdicts = robots_verdicts(user_agent)
        env["robots"] = {k: {"allowed": a, "reason": r} for k, (a, r) in verdicts.items()}
        write_json(os.path.join(out, "environment.json"), env)
        print(f"{env['chrome_product']}; load average {state['load_average']}; results in {out}")
        for run in range(1, args.runs + 1):
            for page in pages:
                allowed, reason = verdicts[page.ident]
                if not allowed:
                    print(f"run {run} {page.ident}: refused by robots.txt ({reason})")
                    continue
                print(f"run {run} {page.ident}: cool-down {args.cooldown} s", flush=True)
                time.sleep(args.cooldown)
                record = census.measure(page.ident, page.url, user_agent, names, TARGET, log, phase_seconds=args.seconds)
                record.update({"run": run, "conditions": system.conditions()})
                write_json(os.path.join(out, f"auto-chrome-{page.ident}-run{run}.json"), record)
                mem = record.get("memory_loaded", {}).get("total_footprint", 0) / 2**20
                print(f"run {run} {page.ident}: {record.get('error') or 'ok'}; {mem:.0f} MB loaded; "
                      f"{record.get('navigation', {}).get('ended_by')}", flush=True)
    summarize(out)


def _prompt(text):
    input(f"\n{text}\n[Return] ")


def _countdown(seconds, label):
    for left in range(seconds, 0, -5):
        print(f"  {label}: {left} s left", flush=True)
        time.sleep(min(5, left))


def index_page(path, urls, pages, mode):
    rows = "\n".join(
        f'<li><a href="{urls[p.ident]}">{p.label}</a><br><small>Interaction, 30 s: {p.interaction}</small></li>'
        for p in pages
    )
    with open(path, "w") as f:
        f.write(f"<!doctype html><meta charset=utf-8><title>S8 corpus ({mode})</title>"
                f"<h1>S8 corpus, {mode}</h1><p>Open one page at a time, as the terminal asks.</p><ol>{rows}</ol>")


def _sample_by_hand(record, key, pids_kinds):
    each, total = system.footprints(pids_kinds, TARGET)
    by_kind = {}
    for pid, kind in pids_kinds.items():
        entry = by_kind.setdefault(kind, {"processes": 0, "footprint": 0})
        entry["processes"] += 1
        entry["footprint"] += each.get(pid, ("?", 0))[1]
    record[key] = {"by_kind": by_kind, "total_footprint": total}


def _idle_by_hand(record, pids_kinds, seconds):
    idle = system.sample_top(pids_kinds, seconds)
    by_kind = {}
    for pid, values in idle.items():
        entry = by_kind.setdefault(pids_kinds[pid], {"cpu_seconds": 0.0, "idle_wakeups": 0, "energy_impact": 0.0})
        entry["cpu_seconds"] = round(entry["cpu_seconds"] + values["cpu_seconds"], 3)
        entry["idle_wakeups"] += values["idle_wakeups"]
        entry["energy_impact"] = round(entry["energy_impact"] + values["energy_impact"], 2)
    record["without_input"] = {"seconds": seconds, "by_kind": by_kind}


def cmd_signin(args):
    profile = os.path.join(SCRATCH, "chrome-profile")
    os.makedirs(profile, exist_ok=True)
    urls = full_urls(URL_LIST)
    index = os.path.join(SCRATCH, "index-signin.html")
    index_page(index, urls, [p for p in PAGES if p.signed_in], "signing in")
    argv = [census.CHROME, f"--user-data-dir={profile}", "--no-first-run", "--no-default-browser-check", "file://" + index]
    print("Chrome Canary opens with the dedicated profile, and no debugging connection.\n"
          "Sign in by hand to each site in the list that you want measured signed in, then quit Canary (Command-Q).")
    subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def cmd_hand(args):
    state = require_conditions(args.allow_battery)
    urls = full_urls(URL_LIST)
    pages = [BY_IDENT[p] for p in args.pages.split(",")] if args.pages else [
        p for p in PAGES if (p.signed_in if args.mode == "signed-in" else (p.logged_out and not p.automated))
    ]
    out = new_results_dir(f"hand-{args.browser}-{args.mode}")
    env = environment()
    env["options"] = {"browser": args.browser, "mode": args.mode, "runs": args.runs, "seconds": args.seconds,
                      "pages": [p.ident for p in pages]}
    write_json(os.path.join(out, "environment.json"), env)
    os.makedirs(SCRATCH, exist_ok=True)
    index = os.path.join(SCRATCH, f"index-{args.mode}.html")
    index_page(index, urls, pages, args.mode)
    print(f"Load average {state['load_average']}. Close other apps you don't need, and keep the Mac otherwise idle.")
    baseline = set()
    if args.browser == "safari":
        _prompt("Quit Safari if it is open (Safari > Quit Safari), then press Return.")
        baseline = set(system.safari_processes())
        window = "a Private window (File > New Private Window)" if args.mode == "logged-out" else "a normal window"
        subprocess.run(["/usr/bin/open", "-a", "Safari", index])
        _prompt(f"Safari opens the list. Use {window} with only the list in it, sized like the census window "
                f"({census.WINDOW[0]} x {census.WINDOW[1]} points) if you can.")
    for run in range(1, args.runs + 1):
        for page in pages:
            record = {"page": page.ident, "run": run, "mode": args.mode, "browser": args.browser,
                      "started": time.strftime("%Y-%m-%dT%H:%M:%S")}
            browser = None
            profile = os.path.join(SCRATCH, "chrome-profile") if args.mode == "signed-in" else os.path.join(
                TARGET, f"hand-profile-{os.getpid()}")
            if args.browser == "chrome":
                os.makedirs(profile, exist_ok=True)
                argv = [census.CHROME, f"--user-data-dir={profile}", "--no-first-run", "--no-default-browser-check",
                        f"--window-size={census.WINDOW[0]},{census.WINDOW[1]}", "--window-position=0,0", "file://" + index]
                browser = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                time.sleep(3)
            kinds = (lambda: system.chrome_processes(browser.pid)) if browser else (
                lambda: system.safari_processes(exclude=baseline))
            _prompt(f"Run {run}: click '{page.label}' in the list (it opens in the same tab). "
                    "When the page looks complete, press Return.")
            _sample_by_hand(record, "memory_loaded", kinds())
            _prompt(f"Now, for 30 s: {page.interaction}. Press Return, then start.")
            _countdown(args.seconds, "interaction")
            _sample_by_hand(record, "memory_after_interaction", kinds())
            print("Now leave the page alone: don't touch the mouse or keyboard for 30 s.", flush=True)
            time.sleep(2)
            current = kinds()
            _idle_by_hand(record, current, args.seconds)
            _sample_by_hand(record, "memory_after", kinds())
            record["conditions"] = system.conditions()
            write_json(os.path.join(out, f"hand-{args.browser}-{args.mode}-{page.ident}-run{run}.json"), record)
            mem = record["memory_loaded"]["total_footprint"] / 2**20
            print(f"  recorded: {mem:.0f} MB when loaded")
            if browser:
                browser.terminate()
                browser.wait(30)
                if args.mode != "signed-in":
                    subprocess.run(["/bin/rm", "-rf", profile])
            else:
                _prompt("Go back to the list (Command-[), then press Return.")
    summarize(out)


def cmd_safari_status(args):
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    driver = subprocess.Popen(["/usr/bin/safaridriver", "-p", str(port)], stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT)
    try:
        time.sleep(2)
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
        body = json.dumps({"capabilities": {"alwaysMatch": {"browserName": "safari"}}})
        connection.request("POST", "/session", body, {"Content-Type": "application/json"})
        answer = json.loads(connection.getresponse().read() or b"{}").get("value", {})
        if "sessionId" in answer:
            connection.request("DELETE", f"/session/{answer['sessionId']}")
            connection.getresponse().read()
            print("Safari accepts WebDriver sessions: remote automation is enabled.")
        else:
            print(f"Safari refused a WebDriver session: {answer.get('error')}: {answer.get('message')}")
    finally:
        driver.terminate()
        driver.wait(10)


def _median(values):
    values = [v for v in values if isinstance(v, (int, float))]
    if not values:
        return None, None
    return statistics.median(values), (min(values), max(values))


def summarize(directory):
    records = {}
    for name in sorted(os.listdir(directory)):
        if name.endswith(".json") and name != "environment.json":
            with open(os.path.join(directory, name)) as f:
                record = json.load(f)
            records.setdefault(record.get("page", name), []).append(record)
    lines, table = [f"S8 results: {directory}", ""], {}
    mb = 2**20
    for page, runs in records.items():
        row = {"runs": len(runs), "errors": [r["error"] for r in runs if r.get("error")]}

        def metric(label, getter, scale=1.0):
            values = []
            for r in runs:
                try:
                    values.append(getter(r) / scale)
                except (KeyError, TypeError):
                    pass
            row[label] = _median(values)

        for phase in ("memory_blank", "memory_loaded", "memory_after_interaction", "memory_after"):
            metric(f"{phase}.total_MB", lambda r, p=phase: r[p]["total_footprint"], mb)
            metric(f"{phase}.renderers_MB", lambda r, p=phase: r[p]["by_kind"]["renderer"]["footprint"], mb)
        for phase in ("memory_loaded", "memory_after_interaction", "memory_after"):
            metric(f"{phase}.total_over_blank_MB",
                   lambda r, p=phase: r[p]["total_footprint"] - r["memory_blank"]["total_footprint"], mb)
            metric(f"{phase}.renderers_over_blank_MB",
                   lambda r, p=phase: r[p]["by_kind"]["renderer"]["footprint"]
                   - r["memory_blank"]["by_kind"]["renderer"]["footprint"], mb)
        metric("memory_loaded.gpu_MB", lambda r: r["memory_loaded"]["by_kind"]["gpu"]["footprint"], mb)
        metric("memory_loaded.browser_MB", lambda r: r["memory_loaded"]["by_kind"]["browser"]["footprint"], mb)
        metric("navigation.seconds", lambda r: r["navigation"]["seconds"])
        metric("idle.cpu_s", lambda r: sum(v["cpu_seconds"] for v in r["without_input"]["by_kind"].values()))
        metric("idle.wakeups", lambda r: sum(v["idle_wakeups"] for v in r["without_input"]["by_kind"].values()))
        metric("idle.energy", lambda r: sum(v["energy_impact"] for v in r["without_input"]["by_kind"].values()))
        metric("timing.fcp_ms", lambda r: r["timing"]["firstContentfulPaint"])
        metric("timing.lcp_ms", lambda r: r["timing"]["largestContentfulPaint"])
        metric("timing.load_ms", lambda r: r["timing"]["load"])
        metric("network.requests", lambda r: r["network"]["requests"])
        metric("network.MB", lambda r: r["network"]["bytes"], mb)
        metric("dom.elements", lambda r: r["dom_loaded"]["elements"])
        features = sorted({f for r in runs for f in r.get("features", [])})
        row["features"] = len(features)
        row["groups"] = {g: len(v) for g, v in usecounters.groups(features).items()}
        row["css_properties"] = len({c for r in runs for c in r.get("css_properties", [])})
        table[page] = row
        lines.append(f"{page} ({len(runs)} runs{'; errors: ' + '; '.join(row['errors']) if row['errors'] else ''})")
        for key, value in row.items():
            if isinstance(value, tuple) and value[0] is not None:
                median, (low, high) = value
                lines.append(f"  {key:34} median {median:10.2f}   range {low:.2f}-{high:.2f}")
        if row["groups"]:
            lines.append(f"  features {row['features']}; groups: {row['groups']}; CSS properties {row['css_properties']}")
    write_json(os.path.join(directory, "summary.json"), table)
    with open(os.path.join(directory, "summary.txt"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print("\n".join(lines))


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog=__doc__.split("\n\n", 1)[1],
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("prepare")
    check = sub.add_parser("check")
    check.add_argument("--seconds", type=int, default=5, help="length of each phase on the test page (default 5)")
    sub.add_parser("robots")
    auto = sub.add_parser("auto")
    auto.add_argument("--runs", type=int, default=3)
    auto.add_argument("--cooldown", type=int, default=60, help="idle seconds before each page (default 60)")
    auto.add_argument("--seconds", type=int, default=30, help="length of the interaction and of the time without input")
    auto.add_argument("--pages", help="comma-separated page names (default: every automated page)")
    auto.add_argument("--allow-battery", action="store_true")
    sub.add_parser("signin")
    hand = sub.add_parser("hand")
    hand.add_argument("--browser", choices=("chrome", "safari"), required=True)
    hand.add_argument("--mode", choices=("logged-out", "signed-in"), required=True)
    hand.add_argument("--runs", type=int, default=3)
    hand.add_argument("--seconds", type=int, default=30)
    hand.add_argument("--pages", help="comma-separated page names (default: the pages the mode measures)")
    hand.add_argument("--allow-battery", action="store_true")
    sub.add_parser("safari-status")
    summary = sub.add_parser("summary")
    summary.add_argument("directory")
    args = parser.parse_args()
    {
        "prepare": cmd_prepare, "check": cmd_check, "robots": cmd_robots, "auto": cmd_auto, "signin": cmd_signin,
        "hand": cmd_hand, "safari-status": cmd_safari_status, "summary": lambda a: summarize(a.directory),
    }[args.command](args)


if __name__ == "__main__":
    main()
