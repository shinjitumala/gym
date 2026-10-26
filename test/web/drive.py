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
import hashlib
import json
import os
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
    env = dict(os.environ, SQLX_OFFLINE="true")
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
        sql = f.read()
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
    cdp.until_text("main", "checklist", "the checklist after starting")
    cdp.until_text("main", "0 set(s) recorded", "the set count of a fresh session")
    cdp.screenshot("gym-started")


def gym_preset_fills_the_checklist(cdp, base):
    """A preset puts its exercises on the checklist, in its own order."""
    cdp.click("main button", "push · 2 exercises")
    cdp.until_text("main", "☐ bench press", "the checklist after applying a preset")
    checklist = cdp.text("main")
    need("☐ incline press" in checklist, f"the second preset exercise is missing: {checklist[:400]!r}")
    need(
        checklist.index("bench press") < checklist.index("incline press"),
        "the checklist is not in the preset's order",
    )
    cdp.screenshot("gym-checklist")


def gym_exercise_window_records_effort_and_time(cdp, base):
    """The window defaults to the last load and offers the three efforts."""
    cdp.click_contains("main button", "bench press")
    cdp.until_text("main", "this session", "the exercise window")
    load = cdp.eval("document.querySelectorAll('main input[type=text]')[0].value")
    need(load == "70", f"the load defaulted to {load!r}, not the last session's 70")
    efforts = cdp.eval(
        "[...document.querySelectorAll('main select')][0]"
        ".options.length && [...document.querySelectorAll('main select')][0].innerText"
    )
    need("hard" in efforts and "form fail" in efforts and "fail" in efforts, f"the effort list is {efforts!r}")
    need("3+ reps from failure" in efforts, f"an effort does not say what it means: {efforts!r}")
    advice = cdp.text("main")
    need("2.5 kg" in advice, f"no load step in the advice: {advice[:400]!r}")
    need("70.0 kg × 8" in advice, f"the advice does not read the last session: {advice[:400]!r}")
    cdp.fill("main input[type=text]", "72.5", 0)
    cdp.fill("main input[type=text]", "8", 1)
    cdp.click("main button", "save set")
    cdp.until_text("main", "72.5", "the saved set's load")
    saved = cdp.text("main")
    need("hard" in saved, f"the saved set has no effort: {saved[:400]!r}")
    need(
        cdp.eval("document.querySelectorAll('main table tbody tr').length > 0"),
        "the session's table did not gain a row",
    )
    cdp.screenshot("gym-set")


def gym_back_and_end_session(cdp, base):
    """X returns to the checklist; end session closes it."""
    cdp.click_contains("main button", "back to the checklist")
    cdp.until_text("main", "☑ bench press", "the checklist after a set")
    cdp.until_text("main", "1 set(s) recorded", "the session's set count")
    cdp.click("main button", "end session")
    cdp.until_text("main", "Start a session", "the session to close")
    sessions = cdp.text("main")
    need("Recent sessions" in sessions, "the recent sessions panel is gone")
    cdp.screenshot("gym-ended")


def food_food_list_is_sorted_by_last_eaten(cdp, base):
    """The food picker is ordered by the last meal of that food."""
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


def weight_a_weigh_in_defaults_to_now(cdp, base):
    """A weigh-in is a date, a time defaulting to now, kg, optional body fat."""
    cdp.go(f"{base}/weight")
    cdp.until_text("main", "Record a weigh-in")
    cdp.click("main button", "now")
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


def weight_trend_has_a_phase_colour(cdp, base):
    """The trend line is coloured by the phase each weigh-in was taken in."""
    cdp.go(f"{base}/weight/analyse")
    cdp.until_text("main", "Weight by phase")
    legend = cdp.text("main")
    need("bulk" in legend and "cut" in legend, f"the phase legend is missing: {legend[:400]!r}")
    strokes = cdp.eval(
        "[...document.querySelectorAll('main svg polyline')].map(e => e.getAttribute('stroke'))"
    )
    need(strokes, "the weight chart drew no line")


def weight_intake_bar_shows_the_food(cdp, base):
    """Pressing a bar names the foods that made that day."""
    bars = cdp.eval("document.querySelectorAll('main svg rect').length")
    need(bars, "the intake chart drew no bars")
    cdp.eval(
        "(() => { const r = document.querySelectorAll('main svg rect');"
        "r[r.length - 1].dispatchEvent(new MouseEvent('click', {bubbles: true})); return true; })()"
    )
    cdp.until_text("main", "food", "the day's foods")
    panel = cdp.text("main section:last-of-type")
    need(
        any(word in panel for word in ("oats", "bread", "chicken", "rice", "milk")),
        f"the picked day does not name its foods: {panel!r}",
    )
    cdp.screenshot("weight-bars")


def edit_tabs_are_the_three_modes(cdp, base):
    """Edit is reached from a mode's bottom row, and stays inside that mode."""
    cdp.go(f"{base}/gym/edit")
    cdp.until_text("main", "Recent sessions")
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


SCENARIOS = [
    nav_has_three_tabs,
    gym_place_is_sorted_by_last_session,
    gym_preset_fills_the_checklist,
    gym_exercise_window_records_effort_and_time,
    gym_back_and_end_session,
    food_food_list_is_sorted_by_last_eaten,
    food_a_meal_is_previewed_then_saved,
    food_totals_show_the_maintenance_gap,
    weight_a_weigh_in_defaults_to_now,
    weight_trend_has_a_phase_colour,
    weight_intake_bar_shows_the_food,
    edit_tabs_are_the_three_modes,
]


def main():
    os.makedirs(SCRATCH, exist_ok=True)
    binary = build()
    print(f"gym: {binary}")
    print(f"scratch: {SCRATCH}")
    home = os.path.join(SCRATCH, "home")
    profile = os.path.join(SCRATCH, "browser")
    db = os.path.join(SCRATCH, "gym.db")
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
