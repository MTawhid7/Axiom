"""The automated census of one page (corpus CO4, CO5, CO9-CO11).

Chrome Canary runs in a fresh profile under target/, driven through the
DevTools Protocol over a pipe, with the census's token appended to its user
agent. One run of a page:
0. sample the processes on about:blank, the baseline;
1. load: navigate, then wait for the load event and 2 s without network
   activity, or 30 s in all; sample the processes;
2. interaction: 30 s of the page's interaction (CO10); sample again;
3. 30 s without input, with `top` sampling every process; sample again.
Features come from the browser's use counters, sizes from the Network
domain's events and from the DOM read in an isolated world, timings from the
page's own Navigation, Paint and Largest Contentful Paint entries, read in the
same isolated world. Nothing runs in the page's own world.
"""

import json
import os
import shutil
import tempfile
import time
import urllib.parse

import cdp
import interact
import system
import usecounters

TOKEN = "AxiomCensus/0.1 (+https://github.com/MTawhid7/Axiom)"
WINDOW = (1280, 800)
CHROME = os.path.join(system.CANARY_APP, "Contents", "MacOS", "Google Chrome Canary")

# Counts over the document and its open shadow roots; closed shadow roots and
# cross-origin frames' documents are out of reach from here.
DOM_SCRIPT = r"""JSON.stringify((() => {
  let elements = 0, depth = 0, openShadowRoots = 0, customElements = 0;
  const stack = document.documentElement ? [[document.documentElement, 1]] : [];
  while (stack.length) {
    const [node, d] = stack.pop();
    elements++;
    if (d > depth) depth = d;
    if (node.localName.includes('-')) customElements++;
    if (node.shadowRoot) { openShadowRoots++; for (const c of node.shadowRoot.children) stack.push([c, d + 1]); }
    for (const c of node.children) stack.push([c, d + 1]);
  }
  let styleSheets = 0, cssRules = 0, crossOriginSheets = 0;
  for (const sheet of document.styleSheets) {
    styleSheets++;
    try { cssRules += sheet.cssRules.length; } catch (e) { crossOriginSheets++; }
  }
  const count = (s) => document.querySelectorAll(s).length;
  return {
    elements, depth, openShadowRoots, customElements, undefinedCustomElements: count(':not(:defined)'),
    frames: count('iframe, frame'), images: document.images.length, videos: count('video'), audios: count('audio'),
    canvases: count('canvas'), svgRoots: document.getElementsByTagNameNS('http://www.w3.org/2000/svg', 'svg').length,
    mathRoots: document.getElementsByTagNameNS('http://www.w3.org/1998/Math/MathML', 'math').length,
    scripts: document.scripts.length, styleSheets, cssRules, crossOriginSheets,
    adoptedStyleSheets: (document.adoptedStyleSheets || []).length,
    viewport: [innerWidth, innerHeight], window: [outerWidth, outerHeight], devicePixelRatio,
    scrollHeight: document.scrollingElement ? document.scrollingElement.scrollHeight : 0,
    title_present: document.title.length > 0, visibility: document.visibilityState, focused: document.hasFocus(),
  };
})())"""

TIMING_SCRIPT = r"""new Promise((resolve) => {
  const nav = performance.getEntriesByType('navigation')[0] || {};
  const paint = {};
  for (const e of performance.getEntriesByType('paint')) paint[e.name] = e.startTime;
  let lcp = null;
  try {
    new PerformanceObserver((list) => { for (const e of list.getEntries()) lcp = e.startTime; })
      .observe({ type: 'largest-contentful-paint', buffered: true });
  } catch (e) {}
  setTimeout(() => resolve(JSON.stringify({
    responseStart: nav.responseStart, domInteractive: nav.domInteractive,
    domContentLoaded: nav.domContentLoadedEventEnd, load: nav.loadEventEnd,
    firstPaint: paint['first-paint'], firstContentfulPaint: paint['first-contentful-paint'],
    largestContentfulPaint: lcp, protocol: nav.nextHopProtocol,
    resourceEntries: performance.getEntriesByType('resource').length,
  })), 200);
})"""

MEDIA_PROPERTIES = {"kVideoDecoderName", "kAudioDecoderName", "kIsPlatformVideoDecoder", "kIsStreaming",
                    "kRendererName", "kVideoTracks", "kAudioTracks"}


class Network:
    """Requests and bytes by resource type, across the page's sessions, and when the network went quiet."""

    def __init__(self):
        self.types, self.mimes, self.protocols = {}, {}, {}
        self.hosts, self.failed, self.websockets = set(), 0, 0
        self.last_activity = time.monotonic()
        self._kind = {}

    def attach(self, browser):
        browser.on("Network.requestWillBeSent", self._request)
        browser.on("Network.responseReceived", self._response)
        browser.on("Network.loadingFinished", self._finished)
        browser.on("Network.loadingFailed", self._failed)
        browser.on("Network.webSocketCreated", self._websocket)

    def _request(self, session, params):
        url = params["request"]["url"]
        if not url.startswith(("http:", "https:")):
            return
        kind = params.get("type", "Other")
        self._kind[(session, params["requestId"])] = kind
        if "redirectResponse" not in params:
            self.types.setdefault(kind, [0, 0])[0] += 1
        self.hosts.add(urllib.parse.urlsplit(url).hostname)
        self.last_activity = time.monotonic()

    def _response(self, session, params):
        kind, response = params.get("type", "Other"), params["response"]
        if kind in ("Image", "Font", "Media", "Script", "Stylesheet", "Document"):
            key = f"{kind} {response.get('mimeType', '?')}"
            self.mimes[key] = self.mimes.get(key, 0) + 1
        if response.get("protocol"):
            self.protocols[response["protocol"]] = self.protocols.get(response["protocol"], 0) + 1
        self.last_activity = time.monotonic()

    def _finished(self, session, params):
        kind = self._kind.get((session, params["requestId"]))
        if kind:
            self.types.setdefault(kind, [0, 0])[1] += int(params.get("encodedDataLength", 0))
        self.last_activity = time.monotonic()

    def _failed(self, session, params):
        if (session, params["requestId"]) in self._kind:
            self.failed += 1
        self.last_activity = time.monotonic()

    def _websocket(self, session, params):
        self.websockets += 1

    def record(self):
        return {
            "by_type": {k: {"requests": v[0], "bytes": v[1]} for k, v in sorted(self.types.items())},
            "requests": sum(v[0] for v in self.types.values()),
            "bytes": sum(v[1] for v in self.types.values()),
            "mime_types": dict(sorted(self.mimes.items())),
            "protocols": self.protocols,
            "hosts": len(self.hosts),
            "failed": self.failed,
            "websockets": self.websockets,
        }


def chrome_argv(profile, user_agent=None, url="about:blank"):
    argv = [CHROME, f"--user-data-dir={profile}", "--remote-debugging-pipe", "--enable-automation",
            "--no-first-run", "--no-default-browser-check", "--use-mock-keychain",
            "--disable-background-networking", "--disable-sync",
            f"--window-size={WINDOW[0]},{WINDOW[1]}", "--window-position=0,0"]
    if user_agent:
        argv.append(f"--user-agent={user_agent}")
    return argv + [url]


def _page_session(browser, timeout=15):
    """Attaches to the first page target, flattened, and returns its session id."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for info in browser.call("Target.getTargets")["targetInfos"]:
            if info["type"] == "page":
                return browser.call("Target.attachToTarget", {"targetId": info["targetId"], "flatten": True})["sessionId"]
        browser.pump(0.2)
    raise cdp.ProtocolError("no page target appeared")


def default_user_agent(log, scratch):
    """The Canary's own user agent, read from a browser on about:blank."""
    profile = tempfile.mkdtemp(prefix="s8-ua-", dir=scratch)
    browser = cdp.Browser(chrome_argv(profile), log)
    try:
        return browser.call("Browser.getVersion")
    finally:
        browser.close()
        shutil.rmtree(profile, ignore_errors=True)


def _evaluator(browser, page):
    def evaluate(expression, await_promise=False):
        frame = browser.call("Page.getFrameTree", {}, page)["frameTree"]["frame"]["id"]
        world = browser.call("Page.createIsolatedWorld", {"frameId": frame, "worldName": "s8"}, page)
        result = browser.call(
            "Runtime.evaluate",
            {"expression": expression, "contextId": world["executionContextId"], "returnByValue": True,
             "awaitPromise": await_promise},
            page,
        )
        if "exceptionDetails" in result:
            return {"error": result["exceptionDetails"].get("text", "exception")}
        return json.loads(result["result"]["value"])

    return evaluate


def _processes(browser, scratch):
    kinds = system.chrome_processes(browser.process.pid)
    errors = []
    each, total = system.footprints(kinds, scratch, errors)
    by_kind = {}
    for pid, kind in kinds.items():
        entry = by_kind.setdefault(kind, {"processes": 0, "footprint": 0, "unmeasured": 0})
        entry["processes"] += 1
        if pid in each:
            entry["footprint"] += each[pid][1]
        else:
            entry["unmeasured"] += 1
    record = {"by_kind": by_kind, "total_footprint": total, "sum_footprint": sum(f for _, f in each.values()),
              "renderers": sorted((each[p][1] for p, k in kinds.items() if k == "renderer" and p in each), reverse=True)}
    if errors:
        record["footprint_errors"] = errors
    others = [pid for pid, kind in kinds.items() if kind == "other"]
    if others:
        table = system.process_table()
        record["other_processes"] = sorted(
            os.path.basename(table[pid][1].split(" --")[0]) if pid in table else "exited" for pid in others)
    updaters = sum(1 for kind in system.chrome_processes(browser.process.pid, updater=True).values() if kind == "updater")
    if updaters:
        record["updater_processes_left_out"] = updaters
    return kinds, record


def _metrics(browser, page):
    return {m["name"]: m["value"] for m in browser.call("Performance.getMetrics", {}, page)["metrics"]}


def measure(page, url, user_agent, names, scratch, log, phase_seconds=30, load_timeout=30, quiet=2.0):
    """One automated run of `page`; returns its record."""
    profile = tempfile.mkdtemp(prefix=f"s8-{page}-", dir=scratch)
    browser = cdp.Browser(chrome_argv(profile, user_agent), log)
    network, targets, media = Network(), {}, {}
    record = {"page": page, "user_agent_token": TOKEN, "started": time.strftime("%Y-%m-%dT%H:%M:%S"),
              "chrome_canary": system.app_version(system.CANARY_APP)}
    try:
        network.attach(browser)

        def attached(session, params):
            kind = params["targetInfo"]["type"]
            targets[kind] = targets.get(kind, 0) + 1
            child = params["sessionId"]
            browser.send("Network.enable", {}, child)
            browser.send("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True, "flatten": True}, child)
            browser.send("Runtime.runIfWaitingForDebugger", {}, child)

        def player_properties(session, params):
            for prop in params.get("properties", []):
                if prop.get("name") in MEDIA_PROPERTIES:
                    media.setdefault(prop["name"], set()).add(str(prop.get("value"))[:200])

        browser.on("Target.attachedToTarget", attached)
        browser.on("Media.playerPropertiesChanged", player_properties)
        session = _page_session(browser)
        for method in ("Page.enable", "Network.enable", "Performance.enable", "Media.enable"):
            browser.call(method, {}, session)
        browser.call("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True, "flatten": True}, session)
        evaluate = _evaluator(browser, session)
        # The same browser on about:blank, settled: the baseline that a page's cost is measured from.
        browser.pump(10)
        _, record["memory_blank"] = _processes(browser, scratch)
        browser.call("Browser.getHistograms", {"query": "Blink.UseCounter", "delta": True})

        start = time.monotonic()
        navigation = browser.call("Page.navigate", {"url": url}, session, timeout=load_timeout)
        record["navigation"] = {"error": navigation.get("errorText")}
        loaded = browser.wait_event("Page.loadEventFired", load_timeout, lambda s, p: s == session) is not None
        browser.pump(max(0.0, load_timeout - (time.monotonic() - start)),
                     until=lambda: time.monotonic() - network.last_activity >= quiet)
        quiet_reached = time.monotonic() - network.last_activity >= quiet
        record["navigation"].update({
            "load_event": loaded,
            "ended_by": "quiet" if quiet_reached else "timeout",
            "seconds": round(time.monotonic() - start, 2),
        })
        record["timing"] = evaluate(TIMING_SCRIPT, await_promise=True)
        record["dom_loaded"] = evaluate(DOM_SCRIPT)
        record["metrics_loaded"] = _metrics(browser, session)
        kinds, record["memory_loaded"] = _processes(browser, scratch)

        viewport = record["dom_loaded"].get("viewport") or list(WINDOW)
        started = time.monotonic()
        try:
            record["interaction"] = interact.run(page, browser, session, evaluate, viewport, phase_seconds)
        except cdp.ProtocolError as error:
            # The run goes on: the time without input and the use counters still count.
            record["interaction"] = {"error": str(error), "after_seconds": round(time.monotonic() - started, 1)}
            browser.pump(max(0.0, phase_seconds - (time.monotonic() - started)))
        _, record["memory_after_interaction"] = _processes(browser, scratch)
        visible_after_interaction = evaluate("JSON.stringify(document.visibilityState)")

        kinds = system.chrome_processes(browser.process.pid)
        idle = system.sample_top(kinds, phase_seconds)
        by_kind = {}
        for pid, values in idle.items():
            entry = by_kind.setdefault(kinds[pid], {"cpu_seconds": 0.0, "idle_wakeups": 0, "energy_impact": 0.0})
            entry["cpu_seconds"] = round(entry["cpu_seconds"] + values["cpu_seconds"], 3)
            entry["idle_wakeups"] += values["idle_wakeups"]
            entry["energy_impact"] = round(entry["energy_impact"] + values["energy_impact"], 2)
        record["without_input"] = {"seconds": phase_seconds, "by_kind": by_kind}
        browser.pump(0.1)
        record["dom_after"] = evaluate(DOM_SCRIPT)
        # A hidden page isn't composited and doesn't take input, so a run is
        # valid only if the page stayed visible at every sample.
        record["visibility"] = [record["dom_loaded"].get("visibility"), visible_after_interaction,
                                record["dom_after"].get("visibility")]
        record["valid"] = all(v == "visible" for v in record["visibility"])
        record["metrics_after"] = _metrics(browser, session)
        _, record["memory_after"] = _processes(browser, scratch)

        histograms = browser.call("Browser.getHistograms", {"query": "Blink.UseCounter", "delta": True})["histograms"]
        by_name = {h["name"]: h for h in histograms}
        record["features"] = usecounters.bucket_names(by_name.get(usecounters.FEATURES, {}), names.get("features", {}))
        record["css_properties"] = usecounters.bucket_names(by_name.get(usecounters.CSS_PROPERTIES, {}), names.get("css", {}))
        record["animated_css_properties"] = usecounters.bucket_names(by_name.get(usecounters.ANIMATED_CSS, {}), names.get("css", {}))
        record["web_features"] = [usecounters.web_features_id(n) for n in usecounters.bucket_names(
            by_name.get(usecounters.WEBDX, {}), names.get("webdx", {})) if n != "PageVisits"]
        record["histograms_seen"] = sorted(by_name)
        record["network"] = network.record()
        record["targets"] = targets
        record["media"] = {k: sorted(v) for k, v in media.items()}
    except cdp.ProtocolError as error:
        record["error"] = str(error)
    finally:
        browser.close()
        shutil.rmtree(profile, ignore_errors=True)
    return record
