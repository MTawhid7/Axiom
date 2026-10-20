"""robots.txt, matched as RFC 9309 specifies (corpus CO3).

- The group for a user agent is the one naming its product token, else `*`;
  groups that name the same agent are merged (§2.2.1).
- The most specific match wins: the rule with the most octets. An allow rule
  wins over an equivalent disallow rule, and no match means allowed (§2.2.2).
- `*` matches any sequence of characters and `$` ends a pattern (§2.2.3).
- A file that is unavailable (status 400-499) allows everything (§2.3.1.3);
  an unreachable one (status 500-599, or no answer) disallows everything
  (§2.3.1.4); every parseable rule is used (§2.3.1.5).
"""

import re
import urllib.error
import urllib.request


def _groups(text):
    groups, current, last_was_agent = [], None, False
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if ":" not in line:
            continue
        key, value = (part.strip() for part in line.split(":", 1))
        key = key.lower()
        if key == "user-agent":
            if not last_was_agent:
                current = {"agents": [], "rules": []}
                groups.append(current)
            current["agents"].append(value.lower())
            last_was_agent = True
        else:
            if key in ("allow", "disallow") and current is not None:
                current["rules"].append((key, value))
            last_was_agent = False
    return groups


def _pattern(path):
    anchored = path.endswith("$")
    body = path[:-1] if anchored else path
    return re.compile("".join(".*" if c == "*" else re.escape(c) for c in body) + ("$" if anchored else ""))


def verdict(text, agent, path):
    """(allowed, reason) for `path` (with its query) and the product token `agent`."""
    groups = _groups(text)
    chosen = [g for g in groups if agent.lower() in g["agents"]] or [g for g in groups if "*" in g["agents"]]
    best = None
    for kind, rule in (r for g in chosen for r in g["rules"]):
        if rule and _pattern(rule).match(path):
            if best is None or len(rule) > len(best[1]) or (len(rule) == len(best[1]) and kind == "allow"):
                best = (kind, rule)
    if best is None:
        return True, "no rule matches"
    return best[0] == "allow", f"{best[0]} {best[1]}"


def check(origin, path, agent, user_agent, timeout=20):
    """Fetches `origin`/robots.txt and returns (allowed, reason) for `path`."""
    request = urllib.request.Request(origin + "/robots.txt", headers={"User-Agent": user_agent})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read(500_000)
            kind = response.headers.get_content_type()
    except urllib.error.HTTPError as error:
        if 400 <= error.code < 500:
            return True, f"no robots file (status {error.code})"
        return False, f"robots file unreachable (status {error.code})"
    except (urllib.error.URLError, TimeoutError) as error:
        return False, f"robots file unreachable ({error})"
    # Every parseable rule counts, whatever the file is served as (§2.3.1.5):
    # an application's HTML page in its place has none.
    allowed, reason = verdict(body.decode("utf-8", "replace"), agent, path)
    return allowed, reason if kind == "text/plain" else f"{reason} (served as {kind})"
