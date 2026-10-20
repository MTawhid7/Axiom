"""A minimal Chrome DevTools Protocol client over `--remote-debugging-pipe`.

Chrome reads commands on file descriptor 3 and writes responses and events on
file descriptor 4. Each message is a JSON object followed by a NUL byte: the
default "ASCIIZ" mode of Chromium's content/browser/devtools/devtools_pipe_handler.cc.
Sessions attached with `flatten` share the pipe, and their messages carry a
`sessionId`. Only the process that launches the browser holds the pipe, so no
other program on the Mac can connect to it, as it could to a debugging port.
"""

import fcntl
import json
import os
import select
import subprocess
import time


class ProtocolError(Exception):
    """A command was answered with an error, or the browser went away."""


class Browser:
    """A browser launched with the pipe; `call` sends a command and waits for its answer."""

    def __init__(self, argv, log):
        command_read, command_write = os.pipe()
        event_read, event_write = os.pipe()
        # The child's ends move above 9 first, so that placing them at 3 and 4
        # can't overwrite one with the other.
        child_read = fcntl.fcntl(command_read, fcntl.F_DUPFD_CLOEXEC, 10)
        child_write = fcntl.fcntl(event_write, fcntl.F_DUPFD_CLOEXEC, 10)
        os.close(command_read)
        os.close(event_write)

        def place_pipe():
            os.dup2(child_read, 3)
            os.dup2(child_write, 4)

        # Every other descriptor of ours is close-on-exec (PEP 446), so only
        # the standard streams and the two pipe ends reach the browser.
        self.process = subprocess.Popen(
            argv, stdin=subprocess.DEVNULL, stdout=log, stderr=log, preexec_fn=place_pipe, close_fds=False
        )
        os.close(child_read)
        os.close(child_write)
        self._write_fd = command_write
        self._read_fd = event_read
        self._buffer = bytearray()
        self._next_id = 0
        self._answers = {}
        self.handlers = {}
        self.closed = False

    def on(self, method, handler):
        """Calls `handler(session, params)` for every event named `method`."""
        self.handlers.setdefault(method, []).append(handler)

    def send(self, method, params=None, session=None):
        self._next_id += 1
        message = {"id": self._next_id, "method": method, "params": params or {}}
        if session:
            message["sessionId"] = session
        data = memoryview(json.dumps(message).encode() + b"\0")
        while data:
            data = data[os.write(self._write_fd, data) :]
        return self._next_id

    def call(self, method, params=None, session=None, timeout=30.0):
        ident = self.send(method, params, session)
        if not self.pump(timeout, until=lambda: ident in self._answers):
            raise ProtocolError(f"{method}: no answer within {timeout} s")
        answer = self._answers.pop(ident)
        if "error" in answer:
            raise ProtocolError(f"{method}: {answer['error'].get('message')}")
        return answer.get("result", {})

    def pump(self, timeout, until=None):
        """Reads and dispatches messages for `timeout` seconds, or until `until()` holds; returns whether it held."""
        deadline = time.monotonic() + timeout
        while True:
            if until is not None and until():
                return True
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return False
            # Short waits, so that a condition on the time alone is checked again.
            ready, _, _ = select.select([self._read_fd], [], [], min(remaining, 0.1))
            if not ready:
                continue
            chunk = os.read(self._read_fd, 1 << 20)
            if not chunk:
                self.closed = True
                raise ProtocolError("the browser closed the pipe")
            self._buffer += chunk
            while (end := self._buffer.find(0)) >= 0:
                message = json.loads(self._buffer[:end])
                del self._buffer[: end + 1]
                self._dispatch(message)

    def wait_event(self, method, timeout, predicate=lambda session, params: True):
        """The parameters of the next event named `method` that `predicate` accepts, or None after `timeout`."""
        caught = []

        def catch(session, params):
            if not caught and predicate(session, params):
                caught.append(params)

        self.on(method, catch)
        try:
            self.pump(timeout, until=lambda: bool(caught))
        finally:
            self.handlers[method].remove(catch)
        return caught[0] if caught else None

    def _dispatch(self, message):
        if "id" in message:
            self._answers[message["id"]] = message
            return
        for handler in list(self.handlers.get(message.get("method"), ())):
            handler(message.get("sessionId"), message.get("params", {}))

    def close(self, timeout=15.0):
        """Asks the browser to close, and kills it if it hasn't exited after `timeout` seconds."""
        if not self.closed and self.process.poll() is None:
            try:
                self.send("Browser.close")
            except OSError:
                pass
        try:
            self.process.wait(timeout)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        for fd in (self._write_fd, self._read_fd):
            try:
                os.close(fd)
            except OSError:
                pass
        self.closed = True
