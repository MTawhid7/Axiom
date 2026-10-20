"""Chrome's use counters: which platform features a page used (S8-Q1).

Blink counts each feature a page uses once per page load, and the browser
records the counts in the histograms `Blink.UseCounter.Features` (bucket = a
`WebFeature` value), `Blink.UseCounter.CSSProperties` and
`Blink.UseCounter.AnimatedCSSProperties` (bucket = a `CSSSampleId` value).
The DevTools Protocol's `Browser.getHistograms` reads them, with `delta` for
one page at a time, so nothing is injected into the page. The enums' names
come from Chromium's mojom files at the browser's version, downloaded into
target/ and read as data.

`Blink.UseCounter.WebDXFeatures` counts the same use by the features of the
W3C WebDX community group's web-features project, whose identifiers the enum's
names encode ("canvas-2d" is `kCanvas_2d`), so each page's features can be
matched to web-features' Baseline status (CP4).
"""

import re

FEATURES = "Blink.UseCounter.Features"
CSS_PROPERTIES = "Blink.UseCounter.CSSProperties"
ANIMATED_CSS = "Blink.UseCounter.AnimatedCSSProperties"
WEBDX = "Blink.UseCounter.WebDXFeatures"

ENUM_FILES = {
    FEATURES: "third_party/blink/public/mojom/use_counter/metrics/web_feature.mojom",
    CSS_PROPERTIES: "third_party/blink/public/mojom/use_counter/metrics/css_property_id.mojom",
    WEBDX: "third_party/blink/public/mojom/use_counter/metrics/webdx_feature.mojom",
}

# S8-Q1's feature groups, matched against WebFeature names (without the `k`).
GROUPS = [
    ("custom elements", r"^(V8)?CustomElementRegistry"),
    ("shadow DOM", r"AttachShadow|DeclarativeShadow"),
    ("canvas 2D", r"Canvas2D|CanvasRenderingContext2D|^HTMLCanvasElement_2D$"),
    ("WebGL", r"WebGL(?!2)"),
    ("WebGL 2", r"WebGL2"),
    ("WebGPU", r"WebGPU|^V8GPU|^HTMLCanvasElement_WebGPU$"),
    ("OffscreenCanvas", r"OffscreenCanvas"),
    ("WebAssembly", r"WebAssembly|Wasm"),
    ("workers", r"^WorkerStart$|^SharedWorkerStart$|DedicatedWorker"),
    ("service workers", r"ServiceWorker"),
    ("IndexedDB", r"IndexedDB|IDBFactory|^V8IDB"),
    ("Web Storage", r"LocalStorage|SessionStorage"),
    ("Cache Storage", r"CacheStorage"),
    ("WebSockets", r"WebSocket"),
    ("WebTransport", r"WebTransport"),
    ("WebRTC", r"RTCPeerConnection"),
    ("Media Source Extensions", r"MediaSource"),
    ("media elements", r"^HTMLMediaElement|^HTMLVideoElement|^HTMLAudioElement"),
    ("WebCodecs", r"WebCodecs"),
    ("Web Animations", r"^V8Element_Animate|^ElementAnimate|^AnimationConstructor|^V8Animation"),
    ("observers", r"IntersectionObserver|ResizeObserver|MutationObserver|PerformanceObserver"),
    ("font loading", r"FontFace"),
    ("image formats", r"WebPImage|AVIFImage|JXL"),
    ("JavaScript modules", r"ImportMap|DynamicImport|ModuleScript"),
    ("MathML", r"MathML"),
    ("dialog and popover", r"DialogElement|Popover"),
    ("container queries", r"ContainerQuer"),
    ("view transitions", r"ViewTransition"),
]


def parse_enum(text):
    """{value: name} for an enum's `kName = value,` entries, without the `k`."""
    return {int(value): name for name, value in re.findall(r"^\s+k(\w+)\s*=\s*(\d+),", text, re.M)}


def web_features_id(name):
    """The web-features identifier for a WebDXFeature name: `Canvas_2dAlpha` is `canvas-2d-alpha`."""
    if name.startswith("DRAFT_"):
        return "draft:" + web_features_id(name[len("DRAFT_"):])
    return re.sub(r"(?<!^)(?=[A-Z])", "-", name).replace("_", "-").replace("--", "-").lower()


def bucket_names(histogram, names):
    """The names of the buckets with a count, for a histogram of one-value buckets."""
    used = []
    for bucket in histogram.get("buckets", []):
        if bucket.get("count", 0) > 0:
            low = bucket.get("low")
            used.append(names.get(low, f"#{low}"))
    return sorted(used)


def groups(features):
    """{group: [feature names]} for the S8-Q1 groups that the features touch."""
    found = {}
    for group, pattern in GROUPS:
        matched = [f for f in features if re.search(pattern, f)]
        if matched:
            found[group] = matched
    return found
