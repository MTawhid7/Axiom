"""The automated pages' interactions (corpus CO10), sent as input events.

Each runs for the given number of seconds through the DevTools Protocol's
Input domain, which delivers events as if from the mouse and keyboard.
Coordinates are in CSS pixels from the viewport's top left.
"""

import math
import time

AT_BOTTOM = (
    "JSON.stringify((() => { const s = document.scrollingElement;"
    " return !s || s.scrollTop + innerHeight >= s.scrollHeight - 2; })())"
)
HAS_WORKBENCH = "JSON.stringify(!!document.querySelector('.monaco-workbench'))"


def _mouse(browser, page, kind, x, y, **extra):
    browser.call("Input.dispatchMouseEvent", {"type": kind, "x": x, "y": y, **extra}, page, timeout=10)


def _wheel(browser, page, x, y, dy):
    _mouse(browser, page, "mouseWheel", x, y, deltaX=0, deltaY=dy)


def _key(browser, page, key, code, keycode, text=None):
    down = {"type": "rawKeyDown", "key": key, "code": code, "windowsVirtualKeyCode": keycode}
    browser.call("Input.dispatchKeyEvent", down, page, timeout=10)
    if text:
        browser.call("Input.dispatchKeyEvent", {"type": "char", "key": key, "text": text}, page, timeout=10)
    up = {"type": "keyUp", "key": key, "code": code, "windowsVirtualKeyCode": keycode}
    browser.call("Input.dispatchKeyEvent", up, page, timeout=10)


def scroll_through(browser, page, evaluate, viewport, seconds, step=100, per_second=20):
    """Wheel down at a steady pace until the end or half the time, then back up."""
    x, y = viewport[0] / 2, viewport[1] / 2
    start = time.monotonic()
    going_down, events = True, 0
    while (elapsed := time.monotonic() - start) < seconds:
        if going_down and (elapsed >= seconds / 2 or (events % 10 == 0 and evaluate(AT_BOTTOM))):
            going_down = False
        _wheel(browser, page, x, y, step if going_down else -step)
        events += 1
        browser.pump(1 / per_second)
    return {"wheel_events": events}


def pan_and_zoom(browser, page, viewport, seconds):
    """Three drags across the map, then two zoom steps in and two out, with pauses for tiles."""
    x, y = viewport[0] / 2, viewport[1] / 2
    start = time.monotonic()
    for dx, dy in ((-300, 0), (0, -200), (250, 150)):
        _mouse(browser, page, "mouseMoved", x, y)
        _mouse(browser, page, "mousePressed", x, y, button="left", buttons=1, clickCount=1)
        for i in range(1, 21):
            _mouse(browser, page, "mouseMoved", x + dx * i / 20, y + dy * i / 20, button="left", buttons=1)
            browser.pump(0.02)
        _mouse(browser, page, "mouseReleased", x + dx, y + dy, button="left", buttons=0, clickCount=1)
        browser.pump(3)
    for dy in (-100, -100, 100, 100):
        _wheel(browser, page, x, y, dy)
        browser.pump(3)
    browser.pump(max(0.0, seconds - (time.monotonic() - start)))
    return {"drags": 3, "zoom_steps": 4}


def circle_pointer(browser, page, viewport, seconds, per_second=60):
    """The pointer circles the viewport's centre, as a hand on a trackpad would."""
    x, y = viewport[0] / 2, viewport[1] / 2
    radius = min(viewport) / 4
    start, moves = time.monotonic(), 0
    while (elapsed := time.monotonic() - start) < seconds:
        angle = elapsed * math.pi / 2
        _mouse(browser, page, "mouseMoved", x + radius * math.cos(angle), y + radius * math.sin(angle))
        moves += 1
        browser.pump(1 / per_second)
    return {"pointer_moves": moves}


def type_in_editor(browser, page, evaluate, seconds, lines=20):
    """Opens a new untitled file from VS Code's command palette and types `lines` lines."""
    start = time.monotonic()
    while not evaluate(HAS_WORKBENCH):
        if time.monotonic() - start > 20:
            return {"error": "the workbench did not appear within 20 s"}
        browser.pump(0.5)
    _key(browser, page, "F1", "F1", 112)
    browser.pump(1.5)
    browser.call("Input.insertText", {"text": "New Untitled Text File"}, page)
    browser.pump(1.5)
    _key(browser, page, "Enter", "Enter", 13, "\r")
    browser.pump(2)
    budget = max(1.0, seconds - (time.monotonic() - start))
    for i in range(lines):
        browser.call("Input.insertText", {"text": f"line {i + 1}: the quick brown fox jumps over the lazy dog"}, page)
        _key(browser, page, "Enter", "Enter", 13, "\r")
        browser.pump(budget / lines)
    browser.pump(max(0.0, seconds - (time.monotonic() - start)))
    return {"lines_typed": lines}


def run(name, browser, page, evaluate, viewport, seconds):
    if name == "wikipedia" or name == "mdn" or name == "testpage":
        return scroll_through(browser, page, evaluate, viewport, seconds)
    if name == "maps":
        return pan_and_zoom(browser, page, viewport, seconds)
    if name == "threejs":
        return circle_pointer(browser, page, viewport, seconds)
    if name == "vscode":
        return type_in_editor(browser, page, evaluate, seconds)
    raise ValueError(f"no automated interaction for {name}")
