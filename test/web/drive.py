#!/usr/bin/env python3
"""Drive `gym web` in a headless browser, one scenario at a time.

The scenarios are the spec: each function is one behaviour a day of using the
site goes through, named after it. A scenario that does not find what it looks
for fails with the line it was on, so a regression says which behaviour broke
rather than that "the UI is broken".

Everything happens in a scratch directory: a fresh database, a config pointing
at it, a server on a free loopback port, and a browser with a throwaway profile.
Nothing is written into the repository, and the browser, the server and the
profile are all removed when the last scenario finishes.

    python3 test/web/drive.py             # build, then drive
    GYM_BIN=target/release/gym python3 test/web/drive.py    # drive a built binary

Python's standard library only: no Playwright, no Node. The browser is spoken to
over the Chrome DevTools Protocol — a WebSocket client is thirty lines — and the
page is driven by evaluating the same DOM calls a click makes.
"""

import base64
import datetime
import hashlib
import json
import os
import re
import secrets
import shutil
import socket
import sqlite3
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HERE = os.path.dirname(os.path.abspath(__file__))
SCRATCH = os.environ.get("FPR_SCRATCH_DIR") or tempfile.mkdtemp(prefix="gym-web-")
DB = os.path.join(SCRATCH, "gym.db")

# Tailwind 4.3 added the first-party scrollbar utilities (scrollbar-thin,
# scrollbar-thumb-*, scrollbar-track-*), so the build pins that release.
TAILWIND_VERSION = "v4.3.3"

BROWSERS = [
    "brave",
    "brave-browser",
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
    "microsoft-edge",
    "vivaldi",
]


# --------------------------------------------------------------- failures


class Failed(Exception):
    """A scenario's expectation did not hold."""


def need(condition, message):
    if not condition:
        raise Failed(message)


# ------------------------------------------------------------------ websocket


class WebSocket:
    """A client socket for one CDP connection: text frames only, masked out."""

    def __init__(self, url):
        _, _, rest = url.partition("://")
        hostport, _, path = rest.partition("/")
        host, _, port = hostport.partition(":")
        self.sock = socket.create_connection((host, int(port)), timeout=30)
        key = base64.b64encode(secrets.token_bytes(16)).decode()
        request = (
            f"GET /{path} HTTP/1.1\r\n"
            f"Host: {hostport}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(request.encode())
        head = b""
        while b"\r\n\r\n" not in head:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise Failed("the browser closed the debugging socket")
            head += chunk
        need(b"101" in head.split(b"\r\n")[0], f"the browser refused the socket: {head[:80]!r}")
        self.rest = head.split(b"\r\n\r\n", 1)[1]

    def send(self, text):
        payload = text.encode()
        header = bytearray([0x81])
        length = len(payload)
        if length < 126:
            header.append(0x80 | length)
        elif length < 65536:
            header.append(0x80 | 126)
            header += struct.pack(">H", length)
        else:
            header.append(0x80 | 127)
            header += struct.pack(">Q", length)
        mask = secrets.token_bytes(4)
        header += mask
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.sock.sendall(bytes(header) + masked)

    def recv(self):
        while True:
            b0 = self._read(2)
            length = b0[1] & 0x7F
            if length == 126:
                length = struct.unpack(">H", self._read(2))[0]
            elif length == 127:
                length = struct.unpack(">Q", self._read(8))[0]
            payload = self._read(length)
            if b0[0] & 0x0F == 0x1:
                return payload.decode()

    def _read(self, n):
        while len(self.rest) < n:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise Failed("the browser closed the debugging socket")
            self.rest += chunk
        out, self.rest = self.rest[:n], self.rest[n:]
        return out

    def close(self):
        try:
            self.sock.close()
        except OSError:
            pass


# ------------------------------------------------------------------------ CDP


class Cdp:
    """One browser, one tab, and the four calls a scenario needs."""

    def __init__(self, port, profile):
        self.port = port
        self.profile = profile
        self.proc = None
        self.ws = None
        self.next_id = 0
        self.session = None
        self.tab = None
        self.last_click = None

    def start(self):
        binary = None
        for name in BROWSERS:
            found = shutil.which(name)
            if found:
                binary = found
                break
        need(binary is not None, f"no browser found on PATH (tried {', '.join(BROWSERS)})")
        self.proc = subprocess.Popen(
            [
                binary,
                "--headless=new",
                f"--remote-debugging-port={self.port}",
                f"--user-data-dir={self.profile}",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-gpu",
                "--disable-extensions",
                "--disable-background-networking",
                "--window-size=1280,900",
                "about:blank",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        deadline = time.time() + 30
        while time.time() < deadline:
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{self.port}/json/version", timeout=2) as r:
                    info = json.load(r)
                self.ws = WebSocket(info["webSocketDebuggerUrl"])
                return
            except (OSError, Failed, KeyError):
                time.sleep(0.2)
        raise Failed("the browser did not open its debugging port")

    def call(self, method, **params):
        self.next_id += 1
        message = {"id": self.next_id, "method": method, "params": params}
        if self.session:
            message["sessionId"] = self.session
        self.ws.send(json.dumps(message))
        while True:
            reply = json.loads(self.ws.recv())
            if reply.get("id") == self.next_id:
                if "error" in reply:
                    raise Failed(f"{method}: {reply['error'].get('message')}")
                return reply.get("result", {})

    def open_tab(self):
        target = self.call("Target.createTarget", url="about:blank")["targetId"]
        self.tab = target
        self.session = self.call("Target.attachToTarget", targetId=target, flatten=True)["sessionId"]
        self.call("Page.enable")
        self.call("Runtime.enable")

    def go(self, url):
        self.call("Page.navigate", url=url)
        self.until("document.readyState === 'complete'", 20, "the page to finish loading")
        # The site hydrates after the module script has fetched the WASM. Until
        # then a click has no handler, so every navigation waits for the fetch
        # to have happened and then gives the mount a moment to attach.
        self.until(
            "performance.getEntriesByType('resource').some(r => r.name.endsWith('.wasm'))"
            " || document.querySelectorAll('script[type=module]').length === 0",
            20,
            "the web bundle to load",
        )
        time.sleep(0.4)

    def eval(self, expression):
        result = self.call(
            "Runtime.evaluate",
            expression=expression,
            returnByValue=True,
            awaitPromise=True,
        )
        if result.get("exceptionDetails"):
            raise Failed(f"javascript: {result['exceptionDetails'].get('text')} in {expression[:120]}")
        return result.get("result", {}).get("value")

    def until(self, expression, timeout, what):
        """Poll for an effect, re-playing the last click until it lands.

        A click can arrive before the WASM has hydrated, in which case nothing
        is listening yet and nothing happens. Re-clicking is what a person does
        when a tap does nothing; the assertion that follows is unchanged, so a
        page that really does not react still fails.
        """
        deadline = time.time() + timeout
        next_tap = time.time() + 1.5
        while time.time() < deadline:
            try:
                if self.eval(expression):
                    return
            except Failed:
                pass
            if self.last_click and time.time() > next_tap:
                try:
                    self.eval(self.last_click)
                except Failed:
                    pass
                next_tap = time.time() + 1.5
            time.sleep(0.1)
        raise Failed(f"waited {timeout}s for {what}")

    def until_text(self, selector, text, what=None):
        expression = (
            f"[...document.querySelectorAll({json.dumps(selector)})]"
            f".map(e => e.innerText).join('\\n').includes({json.dumps(text)})"
        )
        self.until(expression, 10, what or f"{text!r} under {selector}")

    def text(self, selector):
        return self.eval(
            f"[...document.querySelectorAll({json.dumps(selector)})]"
            f".map(e => e.innerText).join('\\n')"
        )

    def click(self, selector, text=None, index=0):
        """Click the first element of `selector` whose text is `text`, retrying.

        Callers cannot know whether the WASM has finished hydrating — the module
        script and the mount are two separate steps — so a click that had no
        handler is retried rather than failed. What it was meant to cause is what
        the scenario asserts, so a retry cannot make a broken page pass.
        """
        finder = (
            "[...document.querySelectorAll(%s)].filter(e => %s)"
            % (
                json.dumps(selector),
                "true" if text is None else f"e.innerText.trim() === {json.dumps(text)}",
            )
        )
        count = self.eval(f"{finder}.length")
        need(
            count,
            f"no {selector} reading {text!r}" if text else f"no {selector} on the page",
        )
        need(index < count, f"only {count} {selector} reading {text!r}")
        self.tap(f"(() => {{ const e = {finder}[{index}]; e.scrollIntoView({{block: 'center'}}); e.click(); return true; }})()")

    def click_contains(self, selector, text, index=0):
        finder = (
            "[...document.querySelectorAll(%s)].filter(e => e.innerText.includes(%s))"
            % (json.dumps(selector), json.dumps(text))
        )
        need(self.eval(f"{finder}.length"), f"no {selector} containing {text!r}")
        self.tap(f"(() => {{ const e = {finder}[{index}]; e.scrollIntoView({{block: 'center'}}); e.click(); return true; }})()")

    def tap(self, expression):
        self.last_click = expression
        self.eval(expression)

    def fill(self, selector, value, index=0):
        """Set a field the way a person does: the value, then the events.

        The forms here read their fields from the DOM on submit, so this is the
        whole of typing. `input` and `change` are both sent because a native
        picker fires `change` and a keystroke fires `input`, and the page listens
        for whichever matches the control.
        """
        self.eval(
            "(() => {"
            f"const e = document.querySelectorAll({json.dumps(selector)})[{index}];"
            "if (!e) return false;"
            "e.focus();"
            f"e.value = {json.dumps(value)};"
            "e.dispatchEvent(new Event('input', {bubbles: true}));"
            "e.dispatchEvent(new Event('change', {bubbles: true}));"
            "return true;"
            "})()"
        )

    def choose(self, selector, value, index=0):
        self.eval(
            "(() => {"
            f"const e = document.querySelectorAll({json.dumps(selector)})[{index}];"
            "if (!e) return false;"
            f"e.value = {json.dumps(value)};"
            "e.dispatchEvent(new Event('change', {bubbles: true}));"
            "return true;"
            "})()"
        )

    def screenshot(self, name):
        data = self.call("Page.captureScreenshot", format="png")["data"]
        path = os.path.join(SCRATCH, f"{name}.png")
        with open(path, "wb") as f:
            f.write(base64.b64decode(data))
        return path

    def close(self):
        if self.ws:
            self.ws.close()
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()


# ---------------------------------------------------------------- the fixture


# How many days before the day the suite runs each of the fixture's rows sits.
# `fixtures.sql` dates a row by naming one of these, so the whole log moves with
# the calendar and a scenario about recency — "yesterday", the range's oldest
# weigh-in — answers the same on any day. The times of day stay in the SQL, next
# to the rows they belong to.
DAYS_AGO = {
    "phase_start": 56,
    "bench_first": 22,
    "bench_second": 15,
    "bench_third": 8,
    "bench_warmup_only": 6,
    "deload": 11,
    "squat_first": 19,
    "squat_second": 12,
    "row_first": 40,
    "row_second": 33,
    "row_warmup_only": 26,
    "preset_used": 2,
    "meal_newest": 1,
    "meal_older": 2,
    "weigh_outside": 140,
    "weigh_1": 56,
    "weigh_2": 49,
    "weigh_3": 42,
    "weigh_4": 35,
    "weigh_5": 28,
    "weigh_6": 21,
    "weigh_7": 14,
    "weigh_8": 7,
    "weigh_9": 3,
}

# `{{on:name}}` is the local day, `{{at:name:HH:MM:SS}}` a local instant on it.
DATE_TOKEN = re.compile(r"\{\{(on|at):(\w+)(?::(\d{2}:\d{2}:\d{2}))?\}\}")


def dated(sql):
    """Fill the fixture's date tokens with days counted back from today.

    The instant carries the machine's own offset for that day, so the server
    reads back the local date the fixture meant — including across a daylight
    saving change, which a fixed offset would shift by an hour.
    """

    def fill(m):
        kind, name, clock = m.groups()
        days = DAYS_AGO.get(name)
        need(
            days is not None,
            f"fixtures.sql dates a row '{{{{{kind}:{name}}}}}' with no day to count back",
        )
        day = datetime.date.today() - datetime.timedelta(days=days)
        if kind == "on":
            return day.isoformat()
        moment = datetime.datetime.combine(day, datetime.time.fromisoformat(clock))
        return moment.astimezone().isoformat()

    filled = DATE_TOKEN.sub(fill, sql)
    need(not DATE_TOKEN.search(filled), "a fixture date token was left unfilled")
    return filled


def weigh_in_days():
    """The local days of the fixture's oldest and newest weigh-in.

    The chart's axis is the data it drew, and the fixture's dates move with the
    day the suite runs, so the scenario reads the days it seeded from the
    database rather than naming a calendar date that would expire. Every writer
    stores an RFC3339 instant with an offset, so the date part of the stored
    string is the local day the row belongs to.
    """
    connection = sqlite3.connect(DB)
    try:
        oldest, newest = connection.execute("SELECT MIN(date), MAX(date) FROM weight").fetchone()
    finally:
        connection.close()
    return oldest[:10], newest[:10]


def build():
    """Build the server and the WASM bundle, unless GYM_BIN names a binary.

    Both halves are built in release. Leptos's debug build of the WASM emits a
    `<!>` marker inside every element to keep positions stable while it renders,
    and the server's debug build does not, so a debug bundle hydrates against a
    DOM without markers and panics on the first element. Release is also what
    `./install.sh` ships, which is what should be walked.
    """
    binary = os.environ.get("GYM_BIN")
    if binary:
        return os.path.abspath(binary)
    env = dict(os.environ, SQLX_OFFLINE="true", LEPTOS_TAILWIND_VERSION=TAILWIND_VERSION)
    steps = []
    if shutil.which("cargo-leptos"):
        steps.append(["cargo", "leptos", "build", "--release"])
    else:
        print("note: cargo-leptos is not installed, so the page will not hydrate", file=sys.stderr)
    steps.append(["cargo", "build", "--release", "--features", "ssr"])
    for step in steps:
        done = subprocess.run(step, cwd=ROOT, env=env)
        need(done.returncode == 0, f"{' '.join(step)} failed")
    return os.path.join(ROOT, "target", "release", "gym")


def fixture(binary, home, db, page_url):
    """An empty database, seeded from the fixture SQL, and a config for it."""
    os.makedirs(os.path.join(home, ".config"), exist_ok=True)
    with open(os.path.join(home, ".config", "gym.toml"), "w") as f:
        f.write(f'db = "{db}"\nremote = "{db}.remote"\n')
    # A scratch directory can survive a failed run, and `gym db init` refuses a
    # file that is already a database; start from nothing either way.
    for path in (db, db + "-wal", db + "-shm", db + ".remote"):
        if os.path.exists(path):
            os.remove(path)
    done = subprocess.run(
        [binary, "db", "init"],
        env=dict(os.environ, HOME=home),
        capture_output=True,
        text=True,
    )
    need(done.returncode == 0, f"gym db init failed: {(done.stderr + done.stdout)[-400:]}")
    with open(os.path.join(HERE, "fixtures.sql")) as f:
        sql = dated(f.read())
    connection = sqlite3.connect(db)
    connection.executescript(sql)
    connection.commit()
    connection.close()


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def serve(binary, home, port):
    proc = subprocess.Popen(
        [binary, "web", "--addr", f"127.0.0.1:{port}"],
        env=dict(os.environ, HOME=home),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    deadline = time.time() + 30
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/status", timeout=2):
                return proc
        except OSError:
            if proc.poll() is not None:
                output = proc.stdout.read().decode()[-800:]
                raise Failed(f"gym web exited before it served anything: {output}")
            time.sleep(0.2)
    proc.terminate()
    raise Failed("gym web did not answer on its port")


# --------------------------------------------------------------- the scenarios


def in_section(cdp, title, expression):
    """Evaluate `expression` with `s` bound to the section titled `title`.

    The edit page keeps every panel in the document and hides the ones that are
    not chosen, so a page-wide ``button[innerText='edit']`` finds the wrong one.
    A section title is what a reader uses to tell them apart, so it is what the
    scenario uses too.
    """
    return cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        f".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === {json.dumps(title)});"
        f"if (!s) return null; return ({expression}); }})()"
    )


def section_click(cdp, title, text, selector="button"):
    ok = cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        f".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === {json.dumps(title)});"
        "if (!s) return false;"
        f"const e = [...s.querySelectorAll({json.dumps(selector)})]"
        f".find(e => e.innerText.trim() === {json.dumps(text)});"
        "if (!e) return false; e.scrollIntoView({block: 'center'}); e.click(); return true; })()"
    )
    need(ok, f"no {selector} reading {text!r} in the {title!r} section")


def section_fill(cdp, title, selector, value, index=0):
    ok = cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        f".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === {json.dumps(title)});"
        "if (!s) return false;"
        f"const e = s.querySelectorAll({json.dumps(selector)})[{index}];"
        "if (!e) return false; e.focus();"
        f"e.value = {json.dumps(value)};"
        "e.dispatchEvent(new Event('input', {bubbles: true}));"
        "e.dispatchEvent(new Event('change', {bubbles: true})); return true; })()"
    )
    need(ok, f"no {selector}[{index}] in the {title!r} section")


def choose_text(cdp, expression, text):
    """Pick the option whose label contains `text` in the select at `expression`."""
    ok = cdp.eval(
        f"(() => {{ const s = {expression}; if (!s) return false;"
        f"const o = [...s.options].find(o => o.textContent.includes({json.dumps(text)}));"
        "if (!o) return false; s.value = o.value;"
        "s.dispatchEvent(new Event('change', {bubbles: true})); return true; })()"
    )
    need(ok, f"no option containing {text!r} at {expression}")


def section_rows(cdp, title, selector):
    """How many elements `selector` matches in the section titled `title`."""
    return in_section(cdp, title, f"s.querySelectorAll({json.dumps(selector)}).length")


def section_pick(cdp, title, text):
    """Click the row of the narrowed list in `title` whose label contains `text`.

    A picker row is two spans, the label and the hint, so its innerText is not a
    single string a scenario can spell out; `contains` is what a reader does
    when they read the list.
    """
    ok = cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        f".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === {json.dumps(title)});"
        "if (!s) return false;"
        f"const e = [...s.querySelectorAll('button')].find(b => b.innerText.includes({json.dumps(text)}));"
        "if (!e) return false; e.scrollIntoView({block: 'center'}); e.click(); return true; })()"
    )
    need(ok, f"no row containing {text!r} in the {title!r} section")


def preset_click(cdp, name, text):
    ok = cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-preset="{name}"]') + ");"
        "if (!rec) return false;"
        f"const e = [...rec.querySelectorAll('button')]"
        f".find(e => e.innerText.trim() === {json.dumps(text)});"
        "if (!e) return false; e.scrollIntoView({block: 'center'}); e.click(); return true; })()"
    )
    need(ok, f"no {text!r} button on the preset {name!r}")


def preset_fill(cdp, name, selector, value, index=0):
    ok = cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-preset="{name}"]') + ");"
        "if (!rec) return false;"
        f"const e = rec.querySelectorAll({json.dumps(selector)})[{index}];"
        "if (!e) return false; e.focus();"
        f"e.value = {json.dumps(value)};"
        "e.dispatchEvent(new Event('input', {bubbles: true}));"
        "e.dispatchEvent(new Event('change', {bubbles: true})); return true; })()"
    )
    need(ok, f"no {selector}[{index}] on the preset {name!r}")


def split_set_day(cdp, form, weekday, preset):
    """Set the preset a split schedules for one weekday, in the form `form`.

    The week is seven rows of a day and a dropdown, so the row is found by the
    day it is: that is what a reader picks by, and it is the only label the row
    has.
    """
    ok = cdp.eval(
        "(() => { const f = document.querySelector(" + json.dumps(form) + ");"
        "if (!f) return false;"
        "const row = [...f.querySelectorAll('div')].find(d =>"
        f"d.innerText.trim().startsWith({json.dumps(weekday)}));"
        "if (!row) return false;"
        "const s = row.querySelector('select'); if (!s) return false;"
        f"const o = [...s.options].find(o => o.textContent.includes({json.dumps(preset)}));"
        "if (!o) return false; s.value = o.value;"
        "s.dispatchEvent(new Event('change', {bubbles: true})); return true; })()"
    )
    need(ok, f"no {weekday} dropdown offering {preset!r} in {form}")


def split_check(cdp, name, value):
    """Tick or untick a split record's `in use` box."""
    ok = cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-split="{name}"]') + ");"
        "if (!rec) return false;"
        "const e = rec.querySelector('input[type=checkbox]'); if (!e) return false;"
        f"e.checked = {'true' if value else 'false'};"
        "e.dispatchEvent(new Event('change', {bubbles: true})); return true; })()"
    )
    need(ok, f"no in-use box on the split {name!r}")


def split_click(cdp, name, text):
    ok = cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-split="{name}"]') + ");"
        "if (!rec) return false;"
        f"const e = [...rec.querySelectorAll('button')]"
        f".find(e => e.innerText.trim() === {json.dumps(text)});"
        "if (!e) return false; e.scrollIntoView({block: 'center'}); e.click(); return true; })()"
    )
    need(ok, f"no {text!r} button on the split {name!r}")


def until_panel_rows(cdp, title, want, what):
    """Wait until one panel's table has `want` rows."""
    cdp.until(
        "(() => { const d = [...document.querySelectorAll('main div')]"
        f".find(e => e.innerText.trim().startsWith({json.dumps(title)}));"
        f"return !!d && d.querySelectorAll('table tbody tr').length === {want}; }})()",
        10,
        what,
    )


def nav_has_three_tabs(cdp, base):
    """The bar is the three things the log records, not a page list."""
    cdp.go(f"{base}/gym")
    tabs = cdp.eval("[...document.querySelectorAll('nav a')].slice(0,3).map(e => e.innerText.trim())")
    need(tabs == ["gym", "food", "weight"], f"the tabs are {tabs}, not gym, food, weight")
    labels = cdp.eval("[...document.querySelectorAll('nav')].map(n => n.innerText.replace(/\\s+/g, ' ').trim())")
    need(
        any("edit" in row and "analyse" in row for row in labels),
        f"no bottom row with edit and analyse: {labels}",
    )
    cdp.go(f"{base}/food")
    labels = cdp.eval("[...document.querySelectorAll('nav')].map(n => n.innerText.replace(/\\s+/g, ' ').trim())")
    need(
        any("edit" in row for row in labels) and not any("analyse" in row for row in labels),
        f"food should reach edit and not analyse: {labels}",
    )
    cdp.screenshot("nav")


def scrollbars_are_thin_and_green(cdp, base):
    """The scrollbar is styled: thin, a green thumb on a black track."""
    cdp.go(f"{base}/gym")
    style = cdp.eval(
        "(() => { const e = document.querySelector('div.bg-black'); const s = getComputedStyle(e);"
        # The border every rule and control uses is green-900, so comparing
        # against it asks the same question without hard-coding a colour space:
        # a browser is free to compute oklch() as lab() or as rgb().
        "return {width: s.scrollbarWidth, colour: s.scrollbarColor,"
        "green: getComputedStyle(document.querySelector('header')).borderTopColor}; })()"
    )
    need(style["width"] == "thin", f"the scrollbar width is {style['width']!r}, not thin")
    thumb, _, track = style["colour"].partition(") ")
    need(
        (thumb + ")") == style["green"],
        f"the scrollbar thumb is {thumb!r}, not the green-900 the borders use ({style['green']!r})",
    )
    need(track == "rgb(0, 0, 0)", f"the scrollbar track is {track!r}, not black")
    scrolls = cdp.eval("getComputedStyle(document.querySelector('main')).scrollbarGutter")
    need(scrolls == "stable", f"the page does not reserve the scrollbar's room: {scrolls!r}")
    cdp.screenshot("scrollbar")


def weight_the_analysis_opens_on_the_data(cdp, base):
    """Weight · analyse opens on a range that holds the log, and draws weight and
    body fat: both are recorded, so a missing line is a bug."""
    cdp.go(f"{base}/weight/analyse")
    cdp.until_text("main", "Weight by phase")
    chosen = cdp.eval(
        "[...document.querySelectorAll('main button.on')].map(e => e.innerText.trim())"
    )
    need(chosen and chosen[0] == "365d", f"the range opens on {chosen!r}, not the year")
    lines = cdp.eval(
        "(() => { const out = {};"
        "document.querySelectorAll('main section').forEach(s => {"
        "const h = s.querySelector('h2');"
        "out[h ? h.innerText : '?'] = s.querySelectorAll('svg polyline').length; });"
        "return out; })()"
    )
    need(lines.get("Weight by phase"), f"the weight line is missing: {lines!r}")
    need(lines.get("Body fat"), f"the body-fat line is missing: {lines!r}")
    need(
        "in the last 365 days" not in cdp.text("main"),
        "the page says the window holds no weigh-ins, so the range is too narrow",
    )
    cdp.screenshot("weight-analysis")


def weight_the_line_is_the_whole_range(cdp, base):
    """The weight line is the range's weigh-ins, not the slope's trailing window.

    The slope needs its 21 days of weigh-ins; the chart is asked about the range
    the reader chose, so a log whose weigh-ins are older than that window still
    has a line, and its oldest point is the range's, not the window's.
    """
    oldest, newest = weigh_in_days()
    cdp.go(f"{base}/weight/analyse")
    cdp.until_text("main", "Weight by phase")
    first, last = cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        ".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === 'Weight by phase');"
        "const t = [...s.querySelectorAll('text.x-tick')].map(e => e.textContent);"
        "return [t[0], t[t.length - 1]]; })()"
    )
    need(
        first == oldest,
        f"the weight line starts at {first!r}, not at the range's oldest weigh-in ({oldest})",
    )
    need(
        last == newest,
        f"the weight line ends at {last!r}, not the newest weigh-in ({newest})",
    )
    # One dot per observation: the line is split where the phase changes, so
    # counting the dots is counting the weigh-ins the chart drew.
    dots = cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        ".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === 'Weight by phase');"
        "return s.querySelectorAll('svg circle').length; })()"
    )
    need(dots >= 10, f"the weight line drew {dots} weigh-ins for the range's ten")
    cdp.screenshot("weight-line-range")


def weight_trend_has_a_phase_colour(cdp, base):
    """The trend line is coloured by the phase each weigh-in was taken in."""
    legend = cdp.text("main")
    need("bulk" in legend and "cut" in legend, f"the phase legend is missing: {legend[:400]!r}")
    strokes = cdp.eval(
        "[...document.querySelectorAll('main svg polyline')].map(e => e.getAttribute('stroke'))"
    )
    need(strokes, "the weight chart drew no line")


def weight_analysis_marks_the_deload_week(cdp, base):
    """The weight line shades the deload week it runs through."""
    weeks = cdp.eval("document.querySelectorAll('main svg rect.deload-week').length")
    need(weeks == 1, f"{weeks} deload bands drawn, not the one deload week")
    need(
        "deload week" in cdp.text("main"),
        "the weight chart's legend does not name the deload band",
    )
    cdp.screenshot("weight-analysis-deload")


def weight_intake_bar_shows_the_food(cdp, base):
    """Pressing a bar names the foods that made that day."""
    bars = cdp.eval("document.querySelectorAll('main svg rect.intake-bar').length")
    need(bars, "the intake chart drew no bars")
    cdp.eval(
        "(() => { const r = document.querySelectorAll('main svg rect.intake-bar');"
        "r[r.length - 1].dispatchEvent(new MouseEvent('click', {bubbles: true})); return true; })()"
    )
    cdp.until_text("main", "food", "the day's foods")
    panel = cdp.text("main section:last-of-type")
    need(
        any(word in panel for word in ("oats", "bread", "chicken", "rice", "milk")),
        f"the picked day does not name its foods: {panel!r}",
    )
    cdp.screenshot("weight-bars")


def weight_foods_are_ranked_over_the_range(cdp, base):
    """Under the bars, the range's foods are ranked in the shown metric."""
    cdp.go(f"{base}/weight/analyse")
    cdp.until_text("#food-ranking", "ranked by calories", "the range's food ranking")
    cdp.until_text("#food-ranking tbody tr", "oats", "the ranked foods")
    calories = cdp.text("#food-ranking tbody")
    need(
        calories.index("oats") < calories.index("chicken") < calories.index("rice"),
        f"the calories are not ranked largest first: {calories!r}",
    )
    cdp.click("main button", "protein")
    cdp.until_text("#food-ranking", "ranked by protein", "the ranking to switch metric")
    protein = cdp.text("#food-ranking tbody")
    need(
        protein.index("chicken") < protein.index("oats"),
        f"the protein ranking is not re-sorted: {protein!r}",
    )
    need("%" in protein, f"the ranking has no share of the range: {protein!r}")
    cdp.screenshot("weight-ranking")


def charts_have_dates_and_numbers(cdp, base):
    """Every chart labels its axes: dates along the bottom, numbers up the side."""
    for page in ("/weight/analyse", "/gym/analyse"):
        cdp.go(f"{base}{page}")
        cdp.until_text("main", "time range")
        charts = cdp.eval(
            "[...document.querySelectorAll('main svg')].map(s => ({"
            "dates: [...s.querySelectorAll('text.x-tick')].map(t => t.textContent),"
            "numbers: [...s.querySelectorAll('text.y-tick')].map(t => t.textContent)}))"
        )
        need(charts, f"{page} drew no chart")
        chart = charts[0]
        need(len(chart["dates"]) >= 3, f"{page} has {len(chart['dates'])} date labels")
        need(
            all(re.fullmatch(r"\d{4}-\d{2}-\d{2}", d) for d in chart["dates"]),
            f"{page} dates are {chart['dates']!r}",
        )
        need(len(chart["numbers"]) >= 2, f"{page} has {len(chart['numbers'])} number labels")
        need(
            all(re.search(r"\d", n) for n in chart["numbers"]),
            f"{page} numbers are {chart['numbers']!r}",
        )
        # An SVG `text` fills black unless told otherwise, and black on this
        # page's black is a label nobody can read - the bug this asserts against.
        fills = cdp.eval(
            "[...document.querySelectorAll('main svg text')]"
            ".map(t => getComputedStyle(t).fill)"
        )
        need(
            fills and not any(f in ("rgb(0, 0, 0)", "none") for f in fills),
            f"{page} axis labels are painted {set(fills)}, which is the background",
        )
        gaps = cdp.eval(
            "(() => { const s = document.querySelector('main svg');"
            "const t = s.querySelector('text.y-tick'); if (!t) return null;"
            "const b = t.getBoundingClientRect(); const svg = s.getBoundingClientRect();"
            "return b.left - svg.left; })()"
        )
        need(
            gaps is None or gaps >= 0,
            f"{page} numbers start {gaps}px left of the chart, so they are clipped",
        )
    cdp.screenshot("chart-axes")


def food_food_list_is_sorted_by_last_eaten(cdp, base):
    """The food picker is ordered by the last meal of that food.

    The fixture eats bread yesterday and the rest two days ago, so the newest
    row's note is "yesterday": the note is written from the food's own date and
    today's, not from the fixture's.
    """
    cdp.go(f"{base}/food")
    cdp.until_text("main", "Record a meal")
    order = cdp.text("main section:nth-of-type(2) div.flex.flex-col button")
    need(order.index("bread") < order.index("chicken"), f"the food list is not newest-first: {order!r}")
    need(order.index("chicken") < order.index("oats"), f"the food list is not newest-first: {order!r}")
    need(order.rstrip().endswith("never eaten"), f"the never-eaten food is not last: {order!r}")
    need("yesterday" in order, f"a food does not say when it was last eaten: {order!r}")


def food_a_meal_is_previewed_then_saved(cdp, base):
    """A row is picked, previewed with its macros, and saved with the others."""
    cdp.click_contains("main section button", "bread")
    # The row's amount is the second text field: the first is the picker's
    # filter, which sits above the draft row.
    cdp.fill("main input[type=text]", "2", 1)
    cdp.until_text("main", "this row: 500 kcal", "the row's preview")
    cdp.click("main button", "add row")
    cdp.until_text("main", "preview: 500 kcal", "the meal's preview")
    cdp.click("main button", "save")
    # The page re-runs its query after a save, so the saved row - not the
    # server's one-line reply, which that reload clears - is what to wait for.
    cdp.until(
        "[...document.querySelectorAll('main section:last-of-type tbody tr td')]"
        ".map(e => e.innerText).join(',').includes('bread')",
        10,
        "the saved meal to appear in the day's table",
    )
    meals = cdp.text("main section:last-of-type")
    need("500" in meals, f"the saved meal's calories are not in the table: {meals!r}")
    cdp.screenshot("food-saved")


def food_totals_show_the_maintenance_gap(cdp, base):
    """The day's totals sit at the top, with the maintenance comparison."""
    cdp.go(f"{base}/food")
    cdp.until_text("main", "Today")
    top = cdp.text("main section:nth-of-type(1)")
    need("calories" in top and "protein" in top, f"no totals at the top: {top!r}")
    need(
        "maintenance" in top.lower(),
        f"the maintenance comparison is missing: {top!r}",
    )


def food_a_preset_loads_the_draft(cdp, base):
    """A preset fills the meal being entered; nothing is written before save."""
    cdp.go(f"{base}/food")
    cdp.until_text("main", "Record a meal")
    saved = "document.querySelectorAll('main section:last-of-type tbody tr').length"
    before = cdp.eval(saved)
    cdp.click_contains("main button", "breakfast ·")
    cdp.until_text("main", "oats · 1.00 ×", "the preset's first food in the draft")
    draft = cdp.text("main section:nth-of-type(2)")
    need("milk · 1.00 ×" in draft, f"the preset's second food is missing: {draft[:600]!r}")
    need(
        cdp.eval(saved) == before,
        "loading a preset wrote meals before save",
    )
    cdp.click("main button", "save")
    cdp.until(
        f"{saved} === {before + 2}",
        10,
        "the preset's two rows to be saved from the draft",
    )
    cdp.screenshot("food-preset")


def food_the_meal_projects_the_day(cdp, base):
    """The meal form projects the day: today's totals plus the rows above it."""
    cdp.go(f"{base}/food")
    cdp.until_text("main", "Record a meal")
    top = int(
        cdp.eval(
            "document.querySelector('main section:nth-of-type(1) span.tabular-nums').innerText"
        )
    )
    cdp.click_contains("main section button", "bread")
    cdp.click("main button", "add row")
    cdp.until_text("#projected-day", "projected day", "the projected day")
    projected = int(
        cdp.eval("document.querySelector('#projected-day span.tabular-nums').innerText")
    )
    need(
        projected == top + 250,
        f"the projected day is {projected} kcal, not today's {top} + the bread's 250",
    )
    cdp.screenshot("food-projection")


def weight_a_weigh_in_defaults_to_now(cdp, base):
    """A weigh-in is a date, a time defaulting to now, kg, optional body fat."""
    cdp.go(f"{base}/weight")
    cdp.until_text("main", "Record a weigh-in")
    cdp.click("main button", "now")
    # The button asks the server for the clock, so the value lands a round trip
    # after the press rather than during the click.
    cdp.until(
        "[...document.querySelectorAll('main input[type=time]')].some(e => e.value !== '')",
        10,
        "the now button to fill the time",
    )
    filled = cdp.eval(
        "[...document.querySelectorAll('main input[type=time]')].map(e => e.value).join(',')"
    )
    need(filled.strip() != "", f"the now button left the time empty: {filled!r}")
    cdp.fill("main input[type=text]", "82.4", 0)
    cdp.fill("main input[type=text]", "17.0", 1)
    cdp.click("main button", "save")
    cdp.until_text("main section:last-of-type", "82.4", "the weigh-in to appear in the list")
    recent = cdp.text("main section:last-of-type")
    need("17.0" in recent, f"the body fat is not in the recent list: {recent!r}")
    cdp.screenshot("weight-saved")


def wait_for_a_new_minute():
    """Sleep until the clock has just crossed a minute, and return that minute.

    The button once wrote the minute the page was rendered with instead of the
    minute it was pressed in, which is what a page left open shows. The two are
    the same value unless the page outlives its minute, so the scenario lets the
    minute end before pressing — that is what makes the assertion able to tell
    them apart, at a cost of at most a minute. It returns the clock at the
    press, which is what the field must hold.
    """
    now = datetime.datetime.now()
    crossing = (now + datetime.timedelta(minutes=1)).replace(second=0, microsecond=0)
    time.sleep((crossing - now).total_seconds() + 0.2)
    return datetime.datetime.now()


def weight_the_now_button_applies_the_press_time(cdp, base):
    """`now` applies the clock of the press, not the clock the page was rendered with."""
    cdp.go(f"{base}/weight")
    cdp.until_text("main", "Record a weigh-in")
    # The page's copy of the clock is fixed from here on, in the minute that is
    # about to end; the press happens in the next one.
    pressed = wait_for_a_new_minute()
    cdp.click("main button", "now")
    cdp.until(
        "[...document.querySelectorAll('main input[type=time]')].some(e => e.value !== '')",
        10,
        "the now button to fill the minute it was pressed in",
    )
    when = cdp.eval(
        "[...document.querySelectorAll('main input[type=time]')].map(e => e.value).join(',')"
    )
    day = cdp.eval(
        "[...document.querySelectorAll('main input[type=date]')].map(e => e.value).join(',')"
    )
    need(
        when == pressed.strftime("%H:%M"),
        f"the now button applied {when!r} to the time, not the press minute {pressed:%H:%M}",
    )
    need(
        day == pressed.strftime("%Y-%m-%d"),
        f"the now button applied {day!r} to the date, not the press day {pressed:%Y-%m-%d}",
    )
    cdp.screenshot("weight-now")


def edit_tabs_are_the_three_modes(cdp, base):
    """Edit is reached from a mode's bottom row, and stays inside that mode."""
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    groups = cdp.eval(
        "[...document.querySelectorAll('main button')].map(e => e.innerText.trim())"
    )
    need("presets" in groups and "places" in groups and "exercises" in groups, f"the gym edit switch is {groups!r}")
    cdp.click("main button", "presets")
    cdp.until_text("main", "A preset is a named list of exercises")
    cdp.until_text("main", "push", "the preset list")
    cdp.go(f"{base}/weight/edit")
    cdp.until_text("main", "Weigh-ins")
    need(
        cdp.eval("document.querySelectorAll('main input[type=date]').length > 0"),
        "the weight edit page has no date field",
    )
    cdp.screenshot("edit")


def gym_place_is_sorted_by_last_session(cdp, base):
    """A session starts by picking a place from a list, newest session first."""
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    order = cdp.text("main section:nth-of-type(2) button")
    need(
        order.index("home") < order.index("the gym"),
        f"the place list is not newest-first: {order!r}",
    )
    need("last session" in order, f"a place does not say when it was last used: {order!r}")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the session's list")
    cdp.until_text("main", "0 set(s) recorded", "the set count of a fresh session")
    cdp.screenshot("gym-started")


def gym_a_session_fills_the_screen(cdp, base):
    """A session in progress is the whole screen, and end session is its exit."""
    box = cdp.eval(
        "(() => { const e = document.querySelector('.session-screen'); if (!e) return null;"
        "const r = e.getBoundingClientRect();"
        "return {w: r.width, h: r.height, sw: window.innerWidth, sh: window.innerHeight,"
        "position: getComputedStyle(e).position}; })()"
    )
    need(box, "the session screen is not on the page")
    need(box["position"] == "fixed", f"the session screen is {box['position']}, not fixed")
    need(
        box["w"] >= box["sw"] - 1 and box["h"] >= box["sh"] - 1,
        f"the session screen is {box['w']}x{box['h']} in a {box['sw']}x{box['sh']} viewport",
    )
    need(
        not cdp.eval("document.body.innerText.includes('Start a session')"),
        "the start form is still behind a session in progress",
    )
    need(
        cdp.eval(
            "!![...document.querySelectorAll('.session-screen button')]"
            ".find(b => b.innerText.trim() === 'end session')"
        ),
        "the session screen has no end session button",
    )
    cdp.screenshot("gym-fullscreen")


def gym_preset_fills_the_list(cdp, base):
    """A preset puts its exercises on the list, in its own order, each with the
    working sets it is there to do."""
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the list after applying a preset")
    checklist = cdp.text("main")
    need("☐ incline press" in checklist, f"the second preset exercise is missing: {checklist[:600]!r}")
    need(
        checklist.index("bench press") < checklist.index("incline press"),
        "the list is not in the preset's order",
    )
    # The count is the preset's own, per exercise: one number for the whole list
    # would pass the same test and tell the reader nothing.
    need(
        "☐ bench press · 0/3 set(s)" in checklist,
        f"the bench press row does not show its three-set target: {checklist[:600]!r}",
    )
    need(
        "☐ incline press · 0/2 set(s)" in checklist,
        f"the incline press row does not show its own two-set target: {checklist[:600]!r}",
    )
    cdp.screenshot("gym-list")


def gym_the_list_is_selected_then_start_sets(cdp, base):
    """Picking an exercise adds it to the list; start sets opens the window."""
    cdp.fill("main input[type=text]", "squat", 0)
    cdp.click_contains("main button", "squat")
    cdp.until_text("main", "☐ squat", "the picked exercise on the list")
    need(
        "this session" not in cdp.text("main"),
        "picking an exercise opened the window instead of adding it to the list",
    )
    cdp.click("main button", "start sets")
    cdp.until_text("main", "this session", "the exercise window")
    cdp.until_text("main", "bench press · chest", "the window on the first exercise to do")
    cdp.screenshot("gym-window")


def panel(cdp, title):
    """One panel of the exercise window, as text.

    The window holds this session's sets beside the recent sessions, and the
    same load × reps can be in both — a set repeated from last week looks the
    same as the one just saved — so an assertion names the panel it is about
    (`this session`, `recent sessions`) rather than the whole page.
    """
    return cdp.eval(
        "(() => { const d = [...document.querySelectorAll('main div')]"
        f".find(e => e.innerText.trim().startsWith({json.dumps(title)}));"
        "return d ? d.innerText : ''; })()"
    )


def until_panel(cdp, title, text, what=None):
    """Wait for a text inside one panel, not anywhere on the page."""
    cdp.until(
        "(() => { const d = [...document.querySelectorAll('main div')]"
        f".find(e => e.innerText.trim().startsWith({json.dumps(title)}));"
        f"return !!d && d.innerText.includes({json.dumps(text)}); }})()",
        10,
        what or f"{text!r} in the {title!r} panel",
    )


def panel_rows(cdp, title):
    """How many table rows one panel shows."""
    return cdp.eval(
        "(() => { const d = [...document.querySelectorAll('main div')]"
        f".find(e => e.innerText.trim().startsWith({json.dumps(title)}));"
        "return d ? d.querySelectorAll('table tbody tr').length : 0; })()"
    )


def gym_a_saved_set_is_edited_in_place(cdp, base):
    """A set saved by mistake is corrected where it is, without leaving the
    session."""
    cdp.fill("main input[type=text]", "70", 0)
    cdp.fill("main input[type=text]", "8", 1)
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "70.0 × 8.0", "the saved set's row")
    cdp.click("main button", "edit")
    cdp.until(
        "document.querySelectorAll('main input[type=text]').length >= 3",
        10,
        "the saved row to fold out",
    )
    cdp.fill("main input[type=text]", "72.5", 0)
    cdp.click("main button", "save")
    until_panel(cdp, "this session", "72.5 × 8.0", "the corrected set's row")
    rows = panel_rows(cdp, "this session")
    need(rows == 1, f"{rows} rows in the session's table after the correction, not one")
    cdp.screenshot("gym-set-edited")


def form_numbers(cdp):
    """The load and reps boxes the exercise window's set row opens on."""
    return cdp.eval(
        "(() => { const i = [...document.querySelectorAll('main input[type=text]')];"
        "return [i[0].value, i[1].value]; })()"
    )


def effort_select(cdp):
    """The set form's effort dropdown: the one select that offers a warmup."""
    return (
        "[...document.querySelectorAll('main select')]"
        ".find(s => [...s.options].some(o => o.textContent.includes('warmup')))"
    )


def gym_the_window_defaults_to_the_last_set_then_the_last_best(cdp, base):
    """The row opens on this session's last working set, and on the last
    session's best set when this session has none of that exercise yet.

    A fresh paint is what is read, so the numbers are the server's pre-fill
    rather than what the previous scenario typed into the box.
    """
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "the list", "the open session's list")
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    load, rep = form_numbers(cdp)
    need(
        (load, rep) == ("72.5", "8"),
        f"the row defaulted to {load} × {rep}, not this session's last set 72.5 × 8",
    )
    # Squat has no set of this session, so the default is the best set of its
    # last session: 102.5 kg × 5, not the phase's rep target.
    choose_text(cdp, "[...document.querySelectorAll('main select')].pop()", "squat")
    cdp.until_text("main", "squat · legs", "the squat window")
    load, rep = form_numbers(cdp)
    need(
        (load, rep) == ("102.5", "5"),
        f"the row defaulted to {load} × {rep}, not the last session's best 102.5 × 5",
    )
    choose_text(cdp, "[...document.querySelectorAll('main select')].pop()", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window again")
    cdp.screenshot("gym-prefill")


def gym_the_next_exercise_alternates(cdp, base):
    """The window names the next exercise in list order — the alternation a
    superset needs — and the dropdown changes it."""
    page = cdp.text("main")
    need(
        "next · incline press" in page,
        f"the next exercise is not the list's next: {page[:800]!r}",
    )
    cdp.click("main button", "next · incline press")
    cdp.until_text("main", "incline press · chest", "the next exercise's window")
    page = cdp.text("main")
    need(
        "next · squat" in page,
        f"the window does not say what follows the second exercise: {page[:800]!r}",
    )
    choose_text(cdp, "[...document.querySelectorAll('main select')].pop()", "bench press")
    cdp.until_text("main", "bench press · chest", "the window the dropdown chose")
    cdp.screenshot("gym-next")


def gym_escape_leaves_the_window_not_the_session(cdp, base):
    """Esc and X leave the exercise window; the session stays open.

    A full-screen session must be interruptible, and its only exit is `end
    session`: a key that closed the session would make a mis-press into a lost
    workout, so Esc does what the window's X does and no more.
    """
    cdp.until_text("main", "this session", "the exercise window")
    cdp.eval(
        "document.dispatchEvent(new KeyboardEvent('keydown',"
        "{key: 'Escape', bubbles: true})); true"
    )
    cdp.until(
        "!document.body.innerText.includes('this session')", 5, "Esc to leave the window"
    )
    need(
        cdp.eval("!!document.querySelector('.session-screen')"),
        "Esc closed the session's screen, not just its window",
    )
    need(
        not cdp.eval("document.body.innerText.includes('Start a session')"),
        "Esc put the start form back, so the session was ended",
    )
    need(
        "the list" in cdp.text("main"),
        f"Esc did not go back to the session's list: {cdp.text('main')[:300]!r}",
    )
    cdp.screenshot("gym-escape")
    # Leave the window open for the scenario that follows.
    cdp.click("main button", "start sets")
    cdp.until_text("main", "this session", "the exercise window again")


def gym_back_and_end_session(cdp, base):
    """X returns to the list; end session closes the session and the screen."""
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☐ bench press · 1/3 set(s)", "the list after a set")
    cdp.until_text("main", "1 set(s) recorded", "the session's set count")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
    need(
        not cdp.eval("!!document.querySelector('.session-screen')"),
        "the session screen is still up after end session",
    )
    need(
        "Recent sessions" not in cdp.text("main"),
        "the recent sessions view is still on the daily page",
    )
    cdp.screenshot("gym-ended")


def gym_a_preset_is_built_by_searching_and_edited(cdp, base):
    """A preset is built by searching for its exercises, and an existing preset
    is rewritten the same way from the edit page."""
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    cdp.click("main button", "presets")
    cdp.until_text("main", "A preset is a named list of exercises")
    added = ("[...document.querySelectorAll('#add-preset button')]"
             ".filter(b => b.innerText.trim() === 'remove').length")
    need(cdp.eval(added) == 0, f"a new preset starts with {cdp.eval(added)} rows, not none")
    cdp.fill("#add-preset input[type=text]", "pull", 0)
    candidates = "div.max-h-64 button"
    everything = section_rows(cdp, "Add a preset", candidates)
    need(everything >= 2, f"the add form lists {everything} exercises, not the log's")
    # "press chest" is no row's text: no single substring carries the words in
    # that order. The search reads them one word at a time, in any order.
    cdp.fill("#add-preset input[type=text]", "press chest", 1)
    narrowed = section_rows(cdp, "Add a preset", candidates)
    need(narrowed == 2, f"'press chest' left {narrowed} exercises, not the two chest presses")
    section_pick(cdp, "Add a preset", "bench press")
    # "sqt" is not a substring of anything either: its letters are in order in
    # "squat", which is what a typo looks like.
    cdp.fill("#add-preset input[type=text]", "sqt", 1)
    need(
        section_rows(cdp, "Add a preset", candidates) == 1,
        "'sqt' did not narrow the list to the one exercise",
    )
    section_pick(cdp, "Add a preset", "squat")
    need(cdp.eval(added) == 2, "the picks did not add the two rows")
    # The count beside a pick is the working sets that exercise is there to do:
    # the second row is set to two, so the saved preset carries two numbers.
    cdp.fill("#add-preset input[type=text]", "2", 3)
    cdp.click("#add-preset button", "add preset")
    cdp.until_text(f'[data-preset="pull"]', "2 exercise(s)", "the new preset")
    need(
        "bench press 3 set(s), squat 2 set(s)"
        in cdp.eval(
            'document.querySelector(\'[data-preset="pull"] span.whitespace-normal\').innerText'
        ),
        "the new preset did not keep the two exercises in order, each with its own target",
    )
    preset_click(cdp, "pull", "edit")
    cdp.until(
        '!!document.querySelector(\'[data-preset="pull"] input[placeholder="filter exercises"]\')',
        10,
        "the preset to fold out with its filter",
    )
    preset_fill(cdp, "pull", "input[type=text]", "pull day", 0)
    preset_fill(cdp, "pull", "input[type=text]", "pll", 1)
    need(
        in_section(
            cdp,
            "Presets",
            "document.querySelector('[data-preset=\"pull\"]')"
            f".querySelectorAll('{candidates}').length",
        )
        == 1,
        "'pll' did not narrow the preset's candidates to the pull up",
    )
    preset_click(cdp, "pull", "pull up · back")
    preset_click(cdp, "pull", "save")
    cdp.until_text('[data-preset="pull day"]', "3 exercise(s)", "the edited preset")
    need(
        "bench press 3 set(s), squat 2 set(s), pull up 3 set(s)"
        in cdp.eval(
            'document.querySelector(\'[data-preset="pull day"] span.whitespace-normal\').innerText'
        ),
        "the edited preset did not keep its three exercises in order, targets and all",
    )
    cdp.screenshot("gym-preset-edited")


def gym_analysis_searches_the_exercises(cdp, base):
    """The analysis exercise list is searchable, and picking one loads it."""
    cdp.go(f"{base}/gym/analyse")
    cdp.until_text("main", "estimated 1RM")
    rows = "s.querySelectorAll('div.flex.flex-col button').length"
    before = in_section(cdp, "Strength and volume", rows)
    need(before and before >= 2, f"the exercise list has {before} rows, not the log's exercises")
    section_fill(cdp, "Strength and volume", "input[type=text]", "squat", 0)
    after = in_section(cdp, "Strength and volume", rows)
    need(after == 1, f"the filter left {after} rows, not the one exercise named squat")
    cdp.eval(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        ".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === 'Strength and volume');"
        "const e = [...s.querySelectorAll('button')].find(b => b.innerText.includes('squat'));"
        "if (!e) return false; e.scrollIntoView({block: 'center'}); e.click(); return true; })()"
    )
    cdp.until(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        ".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === 'Strength and volume');"
        "return s && s.innerText.includes('2 session(s) in the window'); })()",
        10,
        "the chart for the exercise the search picked",
    )
    cdp.screenshot("gym-analysis-search")


def gym_analysis_shades_the_deload_week(cdp, base):
    """The strength chart shades the deload week instead of dropping its point."""
    cdp.go(f"{base}/gym/analyse")
    cdp.until_text("main", "estimated 1RM", "the strength chart")
    weeks = cdp.eval("document.querySelectorAll('main svg rect.deload-week').length")
    need(weeks == 1, f"{weeks} deload bands drawn, not the one deload week")
    panel = cdp.text("main")
    need("deload week" in panel, f"the legend does not say what the band is: {panel[:500]!r}")
    # The band is drawn behind the line, not in place of a point: the strength
    # line still has one point per session in the window.
    counted = re.search(r"(\d+) session\(s\) in the window", panel)
    need(counted, f"the page does not say how many sessions it drew: {panel[:400]!r}")
    drawn = cdp.eval(
        "document.querySelector('main svg polyline[stroke-width=\"2\"]')"
        ".getAttribute('points').trim().split(/\\s+/).length"
    )
    need(
        int(drawn) == int(counted.group(1)),
        f"the strength line has {drawn} points for {counted.group(1)} sessions, so a point was dropped",
    )
    cdp.screenshot("gym-analysis-deload")


def gym_a_deload_is_opted_into(cdp, base):
    """A deload is marked on the session, and its advice halves the load and
    the sets, drawn from the last normal session."""
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    page = cdp.text("main")
    need(
        "weeks since the last deload" in page,
        f"the deload reminder is missing: {page[:400]!r}",
    )
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    need(
        "mark deload" in cdp.text("main") and "un-deload" not in cdp.text("main"),
        "a new session already reads as a deload, so nothing is opted into",
    )
    cdp.click("main button", "mark deload")
    cdp.until_text("main", "un-deload", "the session to be marked a deload")
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the preset's list")
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "deload session", "the exercise window in a deload session")
    advice = cdp.text("main")
    drawn = re.search(
        r"Deload session: ([\d.]+) kg for (\d+) set\(s\) — half of ([\d.]+) kg × (\d+) set\(s\) from (\d{4}-\d{2}-\d{2})",
        advice,
    )
    need(
        drawn is not None,
        f"no deload target drawn from the last normal session: {advice[:600]!r}",
    )
    half_load, half_sets, last_load, last_sets, _ = drawn.groups()
    need(
        abs(float(half_load) - float(last_load) / 2.0) < 0.06,
        f"the deload target {half_load} kg is not half of {last_load} kg",
    )
    need(
        int(half_sets) == max(1, (int(last_sets) + 1) // 2),
        f"the deload target of {half_sets} set(s) is not half of {last_sets}",
    )
    cdp.screenshot("gym-deload")
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "end session")
    cdp.click("main button", "un-deload")
    cdp.until_text("main", "mark deload", "the flag to come back off")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")


def gym_the_list_is_worked_exercise_by_exercise(cdp, base):
    """A session is the list, one exercise at a time: open one, record its sets,
    go back, open the next.

    That loop is the whole of what a round of exercises is; nothing is written
    as a group, so each set stands on its own exercise at its own load.
    """
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the preset's list")
    # The first exercise of the round, prepared with a warmup so a finished
    # session in the log has one to read back later.
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    cdp.fill("main input[type=text]", "40", 0)
    cdp.fill("main input[type=text]", "10", 1)
    choose_text(cdp, effort_select(cdp), "warmup")
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "40.0 × 10.0", "the warmup's row")
    cdp.fill("main input[type=text]", "60", 0)
    cdp.fill("main input[type=text]", "8", 1)
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "60.0 × 8.0", "the bench press working set")
    # Back to the list, which is where the next exercise of the round is picked.
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☐ bench press · 1/3 set(s)", "the list after the first exercise")
    need(
        "☐ incline press" in cdp.text("main"),
        f"the second exercise does not read as still to do: {cdp.text('main')[:600]!r}",
    )
    cdp.click_contains("main button", "incline press")
    cdp.until_text("main", "incline press · chest", "the incline press window")
    cdp.fill("main input[type=text]", "60", 0)
    cdp.fill("main input[type=text]", "10", 1)
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "60.0 × 10.0", "the incline press set")
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☐ incline press · 1/2 set(s)", "the list after the second exercise")
    page = cdp.text("main")
    need(
        "3 set(s) recorded" in page,
        f"the session does not count every set of the round: {page[:500]!r}",
    )
    need(
        "superset" not in page.lower(),
        f"the session's list still offers a superset: {page[:900]!r}",
    )
    cdp.screenshot("gym-list-exercises")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
    # Every set in the log stands on its own exercise: the three are not a group.
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    logged = cdp.text("main")
    for pair in ("40.0 × 10.0", "60.0 × 8.0", "60.0 × 10.0"):
        need(pair in logged, f"{pair} is missing from the log: {logged[:900]!r}")
    need(
        " · ss" not in logged,
        f"the round's sets were written as one superset group: {logged[:900]!r}",
    )


def gym_an_exercise_is_added_by_name(cdp, base):
    """An exercise the log has never had is added to the session's list by name.

    It joins the list like any other exercise, on the day it is first done, and
    arrives with no muscle group until that is filled in on the edit page.
    """
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    cdp.fill("main input.new-exercise", "face pull", 0)
    cdp.click("main button", "add new")
    cdp.until_text("main", "☐ face pull", "the named exercise on the list")
    cdp.click_contains("main button", "☐ face pull")
    cdp.until_text("main", "face pull · ungrouped", "the window on the new exercise")
    cdp.fill("main input[type=text]", "30", 0)
    cdp.fill("main input[type=text]", "12", 1)
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "30.0 × 12.0", "the new exercise's set")
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☑ face pull · 1 set(s)", "the named exercise recorded")
    cdp.screenshot("gym-added-by-name")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
    # The row is in the log now, so the edit page can give it its muscle group.
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    named = cdp.eval(
        "[...document.querySelectorAll('main button, main span, main td')]"
        ".filter(e => e.innerText.trim() === 'face pull').length"
    )
    need(named >= 1, "the exercise added by name is not in the log")


def recent_records(cdp):
    """The exercise window's recent sessions, as text.

    The recent records are per session and carry every set of it, so a warmup is
    read as the warmup it was rather than as a bare load × reps pair among the
    work.
    """
    return panel(cdp, "recent sessions")


def gym_a_warmup_is_not_a_number_to_beat(cdp, base):
    """A warmup is recorded like any other set and is not a rung on the effort
    ladder.

    It is not the last set a session is compared by, not the numbers the next
    row opens on, and not the effort the next row defaults to: a set taken
    nowhere near failure can neither pass for work nor fake progress.
    """
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the preset's list")
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    # The fixture's newest bench session before today was only a warmup, and the
    # advice still compares the last two sessions that did work: 72.5 kg (the
    # session the earlier scenarios recorded) against 70 kg, never the 40 kg
    # warmup in between.
    page = cdp.text("main")
    need(
        "72.5 kg × 8 beat 70.0 kg × 8" in page,
        f"the advice counted the warmup-only session as one to beat: {page[:600]!r}",
    )
    need(
        form_numbers(cdp) == ["72.5", "8"],
        f"the row defaulted to {form_numbers(cdp)}, not the 72.5 kg × 8 to beat",
    )
    # Record the warmup itself: 40 kg × 10, taken nowhere near failure.
    cdp.fill("main input[type=text]", "40", 0)
    cdp.fill("main input[type=text]", "10", 1)
    choose_text(cdp, effort_select(cdp), "warmup")
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "40.0 × 10.0", "the warmup's own row")
    need(
        "warmup" in panel(cdp, "this session"),
        "the warmup is not marked as one in the session's table",
    )
    # A fresh paint, so the assertions read the server's pre-fill: after a
    # warmup the row still points at the working weight, at the working effort.
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "the list", "the open session's list")
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    need(
        form_numbers(cdp) == ["72.5", "8"],
        f"the row opened on the warmup {form_numbers(cdp)}, not on 72.5 × 8",
    )
    need(
        cdp.eval(f"{effort_select(cdp)}.value") == "hard",
        "the effort defaulted to the warmup instead of the working effort",
    )
    page = cdp.text("main")
    need(
        "72.5 kg × 8 beat 70.0 kg × 8" in page,
        f"logging a warmup changed the advice: {page[:600]!r}",
    )
    # The recent records are shown in full: the day's sets with the time each was
    # saved, its load and reps, its effort and its note. The warmup the earlier
    # scenario recorded is there as a warmup, beside the work it prepared for.
    recent = recent_records(cdp)
    need(
        "72.5 × 8.0" in recent,
        f"the recent records do not hold the working set: {recent[:800]!r}",
    )
    need(
        "40.0 × 10.0" in recent,
        f"the recent records drop the warmup: {recent[:800]!r}",
    )
    need(
        "warmup" in recent,
        f"the recent records do not name the warmup's effort: {recent[:800]!r}",
    )
    need(
        re.search(r"\d{2}:\d{2}", recent),
        f"the recent records carry no time of day: {recent[:800]!r}",
    )
    need(
        "note" in recent,
        f"the recent records have no note column: {recent[:800]!r}",
    )
    cdp.screenshot("gym-warmup")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")


def gym_a_warmup_is_not_volume_in_the_analysis(cdp, base):
    """The analysis counts sessions and volume of work: a session that was only
    a warmup is neither a session of the exercise nor a bar of sets per week."""
    cdp.go(f"{base}/gym/analyse")
    cdp.until_text("main", "estimated 1RM")
    section_fill(cdp, "Strength and volume", "input[type=text]", "row", 0)
    offered = in_section(
        cdp,
        "Strength and volume",
        "[...s.querySelectorAll('button')].map(b => b.innerText)"
        ".find(t => t.includes('row · back'))",
    )
    need(offered, "the analysis list does not offer the row")
    need(
        "2 session(s)" in offered,
        f"the row's warmup-only session counted as a session of it: {offered!r}",
    )
    section_pick(cdp, "Strength and volume", "row · back")
    cdp.until(
        "(() => { const s = [...document.querySelectorAll('main section')]"
        ".find(x => x.querySelector('h2') && x.querySelector('h2').innerText === 'Strength and volume');"
        "return s && s.innerText.includes('2 session(s) in the window'); })()",
        10,
        "the row's chart, drawn from its two sessions of work",
    )
    points = cdp.eval(
        "document.querySelector('main svg polyline[stroke-dasharray]')"
        ".getAttribute('points').trim().split(/\\s+/).length"
    )
    need(
        points == 2,
        f"the volume line has {points} weeks for the row's two sessions of work,"
        " so the warmup-only session's week was counted",
    )
    cdp.screenshot("gym-warmup-analysis")


def food_a_preset_is_built_by_searching(cdp, base):
    """A meal preset's foods are picked from a narrowed list, amount and all,
    and the preset then loads on the daily page."""
    cdp.go(f"{base}/food/edit")
    cdp.until_text("main", "Meals · last 120 days")
    cdp.click("main button", "presets")
    cdp.until_text("main", "A preset is a named set of foods")
    cdp.fill("#add-preset input[type=text]", "snack", 0)
    candidates = "div.max-h-64 button"
    everything = section_rows(cdp, "Add a preset", candidates)
    need(everything >= 2, f"the add form lists {everything} foods, not the log's")
    # A typo, not a substring: the letters of "chikn" are in order in "chicken",
    # which is the row the reader meant.
    cdp.fill("#add-preset input[type=text]", "chikn", 1)
    need(
        section_rows(cdp, "Add a preset", candidates) == 1,
        "'chikn' did not narrow the list to the one food",
    )
    section_pick(cdp, "Add a preset", "chicken")
    cdp.fill("#add-preset input[type=text]", "", 1)
    section_pick(cdp, "Add a preset", "oats")
    cdp.fill("#add-preset input[type=text]", "2", 2)
    cdp.click("#add-preset button", "add preset")
    # Saving re-runs the page's query, which reopens the edit page on its first
    # segment: the preset row is read where it is, hidden or not.
    cdp.until(
        "(() => { const r = document.querySelector('[data-preset=\"snack\"]');"
        "return !!r && r.textContent.includes('chicken ×2.0'); })()",
        10,
        "the new preset's first food",
    )
    need(
        "oats ×1.0"
        in cdp.eval('document.querySelector(\'[data-preset="snack"]\').textContent'),
        "the new preset did not keep both foods, in the order they were picked",
    )
    cdp.screenshot("food-preset-search")
    cdp.go(f"{base}/food")
    cdp.until_text("main", "snack ·", "the new preset on the daily page")


def preset_food_rows(cdp, name):
    """The preset's folded-out foods: `name ×amount`, in the order shown."""
    return cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-preset="{name}"]') + ");"
        "if (!rec) return null;"
        "return [...rec.querySelectorAll('div.border-dashed')]"
        ".map(r => r.querySelector('span').innerText + ' ×' + r.querySelector('input').value); })()"
    )


def preset_row_click(cdp, name, row, text):
    """Click the button reading `text` on row `row` of the preset `name`."""
    ok = cdp.eval(
        "(() => { const rec = document.querySelector(" + json.dumps(f'[data-preset="{name}"]') + ");"
        "if (!rec) return false;"
        f"const r = [...rec.querySelectorAll('div.border-dashed')][{row}];"
        "if (!r) return false;"
        f"const e = [...r.querySelectorAll('button')]"
        f".find(e => e.innerText.trim() === {json.dumps(text)});"
        "if (!e) return false; e.click(); return true; })()"
    )
    need(ok, f"no {text!r} button on row {row} of the preset {name!r}")


def food_a_preset_is_edited_in_place(cdp, base):
    """A preset's row folds out into its own foods: it is renamed, one food is
    dropped, another added and an amount retyped, and the foods keep the order
    they were in, which is the order the daily page loads them in."""
    cdp.go(f"{base}/food/edit")
    cdp.until_text("main", "Meals · last 120 days")
    cdp.click("main button", "presets")
    cdp.until_text("main", "A preset is a named set of foods")
    preset_click(cdp, "breakfast", "edit")
    opened = preset_food_rows(cdp, "breakfast")
    need(
        opened == ["oats ×1.0", "milk ×1.0"],
        f"the preset folded out as {opened!r}, not its own two foods in its own order",
    )
    preset_fill(cdp, "breakfast", "div.border-dashed input[type=text]", "2", 0)
    preset_row_click(cdp, "breakfast", 1, "remove")
    preset_fill(cdp, "breakfast", "input[type=text]", "porridge", 0)
    preset_fill(cdp, "breakfast", 'input[placeholder="filter foods"]', "bread")
    section_pick(cdp, "Presets", "bread")
    preset_fill(cdp, "breakfast", "div.border-dashed input[type=text]", "1.5", 1)
    need(
        preset_food_rows(cdp, "breakfast") == ["oats ×2", "bread ×1.5"],
        f"the form reads {preset_food_rows(cdp, 'breakfast')!r} before saving",
    )
    preset_click(cdp, "breakfast", "save")
    # Saving re-runs the page's query, which reopens the edit page on its first
    # segment: the renamed row is read where it is, hidden or not.
    cdp.until(
        "(() => { const r = document.querySelector('[data-preset=\"porridge\"]');"
        "return !!r && r.textContent.includes('oats ×2.0, bread ×1.5'); })()",
        10,
        "the renamed preset with its two foods, in the order they were left in",
    )
    cdp.go(f"{base}/food")
    cdp.until_text("main", "porridge ·", "the edited preset on the daily page")
    cdp.click_contains("main button", "porridge ·")
    cdp.until_text("main", "oats · 2.00 ×", "the edited preset's first food in the draft")
    draft = cdp.text("main section:nth-of-type(2)")
    rows = (draft.find("oats · 2.00 ×"), draft.find("bread · 1.50 ×"))
    need(
        -1 not in rows and rows[0] < rows[1],
        f"the draft is not in the preset's order: {draft[:600]!r}",
    )
    cdp.screenshot("food-preset-edited")


def watch(cdp):
    """Start counting the frames the page spends on its loading line.

    A save re-reads the page, and what the reader must not see is the page
    dropping back to `Loading…` and rebuilding: the mutation observer samples
    after every batch of DOM changes, so a fallback that appears for one frame
    is still counted.
    """
    cdp.eval(
        "(() => {"
        "window.__blank = 0;"
        "const check = () => {"
        "if (document.body && document.body.innerText.includes('Loading\\u2026'))"
        "window.__blank += 1;"
        "};"
        "window.__watch = new MutationObserver(check);"
        "window.__watch.observe(document.body,"
        "{childList: true, subtree: true, characterData: true});"
        "check();"
        "return true;"
        "})()"
    )


def blank_frames(cdp):
    """How many sampled frames showed the loading line since `watch`."""
    cdp.eval("window.__watch.disconnect(); true")
    return cdp.eval("window.__blank")


def sets_recorded(cdp):
    """The session's set total, as the bar over it reads it."""
    found = re.search(r"(\d+) set\(s\) recorded", cdp.text("main"))
    need(found is not None, f"no set total in the session's bar: {cdp.text('main')[:200]!r}")
    return int(found.group(1))


def food_a_save_keeps_the_page(cdp, base):
    """Saving a meal leaves the page where it was: the form, the totals and the
    day's table stay, and the new row arrives in place."""
    cdp.go(f"{base}/food")
    cdp.until_text("main", "Today")
    cdp.click_contains("main section button", "bread")
    cdp.fill("main input[type=text]", "1", 1)
    cdp.until_text("main", "this row: 250 kcal", "the row's preview")
    cdp.click("main button", "add row")
    watch(cdp)
    cdp.click("main button", "save")
    cdp.until_text("main section:last-of-type", "bread", "the saved meal")
    blanks = blank_frames(cdp)
    need(blanks == 0, f"the page fell back to its loading line on {blanks} frame(s)")
    need("Today" in cdp.text("main"), "the day's totals did not survive the save")
    cdp.screenshot("food-save-keeps-page")


def gym_a_saved_set_keeps_the_window(cdp, base):
    """Saving a set re-reads the window, not the page: the table of this
    session's sets grows where it is and the bar's total follows."""
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the checklist of a new session")
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the preset's list")
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    rows = panel_rows(cdp, "this session")
    total = sets_recorded(cdp)
    cdp.fill("main input[type=text]", "75", 0)
    cdp.fill("main input[type=text]", "6", 1)
    watch(cdp)
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "75.0 × 6.0", "the saved set's row")
    blanks = blank_frames(cdp)
    need(blanks == 0, f"the window fell back to its loading line on {blanks} frame(s)")
    need(
        "this session" in cdp.text("main"),
        "saving a set did not keep the exercise window on the page",
    )
    need(
        panel_rows(cdp, "this session") == rows + 1,
        f"the window shows {panel_rows(cdp, 'this session')} rows, not the {rows} it had plus one",
    )
    need(
        sets_recorded(cdp) == total + 1,
        f"the bar reads {sets_recorded(cdp)} set(s), not the {total} it read plus one",
    )
    cdp.screenshot("gym-set-keeps-window")
    # The session is closed, so the scenarios after this one start from the
    # daily page rather than from a session this one left open.
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
# The weekdays in the order the app writes them: 1 is Monday.
WEEKDAYS = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
]


def gym_the_list_counts_working_sets_against_the_target(cdp, base):
    """The list counts the work done against the target the preset gave it, and
    a warmup is not one of the sets counted.

    The count has to be the preset's own, per exercise: the push preset asks the
    bench press for three sets and the incline press for two, so a page showing
    one number for the list cannot pass.
    """
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press · 0/3 set(s)", "the list with the preset's targets")
    need(
        "☐ incline press · 0/2 set(s)" in cdp.text("main"),
        f"the second exercise does not show its own target: {cdp.text('main')[:600]!r}",
    )
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "bench press · chest", "the bench press window")
    # The working numbers, read before the warmup is typed over them: a saved
    # warmup must leave the row pointing back at them, not at itself.
    working = form_numbers(cdp)
    working_effort = cdp.eval(f"{effort_select(cdp)}.value")
    # A warmup is recorded like any other set and counts toward nothing: after
    # one, the exercise has still not started, and the row is back on the work.
    cdp.fill("main input[type=text]", "40", 0)
    cdp.fill("main input[type=text]", "10", 1)
    choose_text(cdp, effort_select(cdp), "warmup")
    cdp.click("main button", "save set")
    until_panel(cdp, "this session", "40.0 × 10.0", "the warmup's row")
    need(
        form_numbers(cdp) == working,
        f"after the warmup the row reads {form_numbers(cdp)}, not the work it prepared for: {working}",
    )
    need(
        cdp.eval(f"{effort_select(cdp)}.value") == working_effort,
        "the row's effort stayed on the warmup instead of the working effort",
    )
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☐ bench press · 0/3 set(s)", "the list after a warmup alone")
    # The working sets are what the target counts, one row at a time; the row is
    # ticked only when the third one lands, because ☐ and ☑ say what is left.
    for done in (1, 2, 3):
        cdp.click_contains("main button", "bench press")
        cdp.until_text("main", "bench press · chest", "the window for the next set")
        cdp.fill("main input[type=text]", "60", 0)
        cdp.fill("main input[type=text]", "8", 1)
        cdp.click("main button", "save set")
        until_panel_rows(
            cdp, "this session", done + 1, f"the {done}th working set's row"
        )
        cdp.click_contains("main button", "back to the checklist")
        mark = "☑" if done == 3 else "☐"
        cdp.until_text(
            "main",
            f"{mark} bench press · {done}/3 set(s)",
            f"the list after {done} working set(s)",
        )
    # The other exercise is untouched by any of it: its own count is still zero.
    need(
        "☐ incline press · 0/2 set(s)" in cdp.text("main"),
        f"the second exercise's count moved with the first: {cdp.text('main')[:600]!r}",
    )
    cdp.screenshot("gym-set-target")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")


def gym_a_split_offers_the_days_preset(cdp, base):
    """A split maps weekdays to presets, and a session started on one of those
    days is offered the day's preset rather than being filled with it.

    The weekday is today's, read from the clock the suite runs on: the offer is
    about the session's own day, so pinning it to a fixed day of the week would
    be a scenario that only passes on six days out of seven.
    """
    today = WEEKDAYS[datetime.date.today().weekday()]
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    cdp.click("main button", "splits")
    cdp.until_text("main", "A split names the preset each weekday asks for")
    cdp.fill("#add-split input[type=text]", "week", 0)
    split_set_day(cdp, "#add-split", today, "push")
    cdp.eval(
        "document.querySelector('#add-split input[type=checkbox]').checked = true"
    )
    cdp.click("#add-split button", "add split")
    cdp.click("main button", "splits")
    cdp.until(
        "(() => { const r = document.querySelector('[data-split=\"week\"]');"
        f"return !!r && r.innerText.includes('in use')"
        f"&& r.innerText.includes({json.dumps(today)}); }})()",
        10,
        "the new split, in use, with its weekday set",
    )
    cdp.screenshot("gym-split")
    # A session started on that day is offered the preset, and nothing is put on
    # the list until the offer is taken up.
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "asks for push", "the offer of the day's preset")
    need(
        "Nothing on the list yet" in cdp.text("main"),
        f"the split filled the list before the offer was answered: {cdp.text('main')[:600]!r}",
    )
    cdp.click("main button", "apply push")
    cdp.until_text("main", "☐ bench press · 0/3 set(s)", "the list the offer filled")
    need(
        not cdp.eval("!!document.querySelector('[data-split-offer]')"),
        "the offer is still up after the list was filled",
    )
    cdp.screenshot("gym-split-offer")
    # The split is the switch: with none in use there is no offer, and the day's
    # preset is still on the session, because the list was filled by a press.
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Edit a set's load")
    cdp.click("main button", "splits")
    cdp.until_text("main", "A split names the preset each weekday asks for")
    split_click(cdp, "week", "edit")
    cdp.until(
        "!!document.querySelector('[data-split=\"week\"] input[type=checkbox]')",
        10,
        "the split's editor",
    )
    split_check(cdp, "week", False)
    split_click(cdp, "week", "save")
    cdp.click("main button", "splits")
    cdp.until(
        "(() => { const r = document.querySelector('[data-split=\"week\"]');"
        "return !!r && !r.innerText.includes('in use'); })()",
        10,
        "the split to be turned off",
    )
    cdp.go(f"{base}/gym")
    cdp.until_text("main", "Start a session")
    cdp.click("main button", "start session")
    cdp.until_text("main", "the list", "the list after starting")
    need(
        not cdp.eval("!!document.querySelector('[data-split-offer]')"),
        "a session was offered a preset with no split in use",
    )
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")


SCENARIOS = [
    nav_has_three_tabs,
    scrollbars_are_thin_and_green,
    weight_the_analysis_opens_on_the_data,
    weight_the_line_is_the_whole_range,
    weight_trend_has_a_phase_colour,
    weight_analysis_marks_the_deload_week,
    weight_intake_bar_shows_the_food,
    weight_foods_are_ranked_over_the_range,
    charts_have_dates_and_numbers,
    food_food_list_is_sorted_by_last_eaten,
    food_a_meal_is_previewed_then_saved,
    food_totals_show_the_maintenance_gap,
    food_a_preset_loads_the_draft,
    food_the_meal_projects_the_day,
    weight_a_weigh_in_defaults_to_now,
    weight_the_now_button_applies_the_press_time,
    edit_tabs_are_the_three_modes,
    gym_place_is_sorted_by_last_session,
    gym_a_session_fills_the_screen,
    gym_preset_fills_the_list,
    gym_the_list_is_selected_then_start_sets,
    gym_a_saved_set_is_edited_in_place,
    gym_the_window_defaults_to_the_last_set_then_the_last_best,
    gym_the_next_exercise_alternates,
    gym_escape_leaves_the_window_not_the_session,
    gym_back_and_end_session,
    gym_a_preset_is_built_by_searching_and_edited,
    gym_analysis_searches_the_exercises,
    gym_analysis_shades_the_deload_week,
    gym_a_warmup_is_not_volume_in_the_analysis,
    gym_a_deload_is_opted_into,
    gym_the_list_is_worked_exercise_by_exercise,
    gym_an_exercise_is_added_by_name,
    gym_a_warmup_is_not_a_number_to_beat,
    food_a_preset_is_built_by_searching,
    food_a_preset_is_edited_in_place,
    food_a_save_keeps_the_page,
    gym_a_saved_set_keeps_the_window,
    gym_the_list_counts_working_sets_against_the_target,
    gym_a_split_offers_the_days_preset,
]


def main():
    os.makedirs(SCRATCH, exist_ok=True)
    binary = build()
    print(f"gym: {binary}")
    print(f"scratch: {SCRATCH}")
    home = os.path.join(SCRATCH, "home")
    profile = os.path.join(SCRATCH, "browser")
    db = DB
    for path in (home, profile):
        os.makedirs(path, exist_ok=True)
    fixture(binary, home, db, None)
    port = free_port()
    server = serve(binary, home, port)
    browser = Cdp(free_port(), profile)
    failures = []
    try:
        browser.start()
        browser.open_tab()
        base = f"http://127.0.0.1:{port}"
        for scenario in SCENARIOS:
            name = scenario.__name__.replace("_", " ")
            try:
                scenario(browser, base)
                print(f"ok    {name}")
            except Failed as e:
                print(f"FAIL  {name}: {e}")
                try:
                    browser.screenshot("failed-" + scenario.__name__)
                except Failed:
                    pass
                failures.append(name)
    finally:
        browser.close()
        server.terminate()
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
    if failures:
        print(f"\n{len(failures)} scenario(s) failed: {', '.join(failures)}")
        return 1
    print(f"\nall {len(SCENARIOS)} scenarios passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
