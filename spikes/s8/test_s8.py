"""S8's offline tests: python3 -I spikes/s8/test_s8.py"""

import os
import sys
import tempfile
import unittest
import urllib.parse

sys.dont_write_bytecode = True  # no __pycache__ beside the spike's sources
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import corpus  # noqa: E402
import robots  # noqa: E402
import system  # noqa: E402
import usecounters  # noqa: E402


class Robots(unittest.TestCase):
    def test_longest_match_wins(self):
        text = "User-agent: *\nDisallow: /maps/\nAllow: /maps/search/\n"
        self.assertEqual(robots.verdict(text, "AxiomCensus", "/maps/search/Times+Square"), (True, "allow /maps/search/"))
        self.assertEqual(robots.verdict(text, "AxiomCensus", "/maps/place/x"), (False, "disallow /maps/"))

    def test_allow_wins_a_tie(self):
        text = "User-agent: *\nDisallow: /page\nAllow: /page\n"
        self.assertTrue(robots.verdict(text, "AxiomCensus", "/page")[0])

    def test_wildcards_and_end_anchor(self):
        text = "User-agent: *\nDisallow: /make/*\nDisallow: /login$\n"
        self.assertFalse(robots.verdict(text, "AxiomCensus", "/make/abc?p=f")[0])
        self.assertFalse(robots.verdict(text, "AxiomCensus", "/login")[0])
        self.assertTrue(robots.verdict(text, "AxiomCensus", "/login/help")[0])

    def test_named_group_overrides_star(self):
        text = "User-agent: *\nDisallow: /\n\nUser-agent: axiomcensus\nAllow: /\n"
        self.assertTrue(robots.verdict(text, "AxiomCensus", "/anything")[0])

    def test_agents_sharing_a_group(self):
        text = "User-agent: other\nUser-agent: *\nDisallow: /search\n"
        self.assertFalse(robots.verdict(text, "AxiomCensus", "/search?q=x")[0])

    def test_no_rules_allow(self):
        self.assertTrue(robots.verdict("<!doctype html><title>An app</title>", "AxiomCensus", "/")[0])
        self.assertTrue(robots.verdict("User-agent: *\nDisallow:\n", "AxiomCensus", "/x")[0])


class Top(unittest.TestCase):
    SAMPLE = """Processes: 493 total, 2 running
PID    %CPU TIME     IDLEW POWER
101    0.0  00:01.50 3     0.2
202    12.5 1:02:03  120   4.5
Processes: 493 total, 2 running
PID    %CPU TIME     IDLEW POWER
101    1.0  00:01.70 5+    0.4
202    10.0 1:02:04  1.2K  3.5
"""

    def test_parse(self):
        samples = system.parse_top(self.SAMPLE)
        self.assertEqual(len(samples), 2)
        self.assertEqual(samples[0][101], (0.0, 1.5, 3, 0.2))
        self.assertEqual(samples[0][202][1], 3723.0)
        self.assertEqual(samples[1][202][2], 1200)
        self.assertEqual(samples[1][101][2], 5)


class Enums(unittest.TestCase):
    def test_parse_enum(self):
        text = "enum WebFeature {\n  kPageDestruction = 0,\n  kOBSOLETE_X = 1,\n  kWorkerStart = 4,\n};\n"
        self.assertEqual(usecounters.parse_enum(text), {0: "PageDestruction", 1: "OBSOLETE_X", 4: "WorkerStart"})

    def test_web_features_ids(self):
        for name, ident in (("ViewTransitions", "view-transitions"), ("Canvas_2d", "canvas-2d"),
                            ("Canvas_2dAlpha", "canvas-2d-alpha"), ("Float16array", "float16array")):
            self.assertEqual(usecounters.web_features_id(name), ident)

    def test_buckets(self):
        histogram = {"buckets": [{"low": 4, "high": 5, "count": 1}, {"low": 0, "high": 1, "count": 0}]}
        self.assertEqual(usecounters.bucket_names(histogram, {4: "WorkerStart"}), ["WorkerStart"])


class Corpus(unittest.TestCase):
    def test_thirteen_pages_five_automated(self):
        self.assertEqual(len(corpus.PAGES), 13)
        self.assertEqual({p.ident for p in corpus.PAGES if p.automated}, {"wikipedia", "maps", "mdn", "threejs", "vscode"})

    def test_no_identifying_parameters(self):
        for page in corpus.PAGES:
            parameters = urllib.parse.parse_qs(urllib.parse.urlsplit(page.url).query)
            self.assertFalse({"u", "t"} & set(parameters), page.ident)

    def test_full_urls_must_match_the_pages(self):
        with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
            f.write("# comment\n" + "\n".join(p.url for p in corpus.PAGES) + "\n")
        try:
            self.assertEqual(corpus.full_urls(f.name)["vscode"], "https://vscode.dev/")
        finally:
            os.remove(f.name)


class Visibility(unittest.TestCase):
    def test_a_run_counts_only_if_visible_throughout(self):
        import s8

        self.assertTrue(s8.visible_throughout({"valid": True}))
        self.assertFalse(s8.visible_throughout({"valid": False}))
        self.assertFalse(s8.visible_throughout({"dom_loaded": {"visibility": "hidden"}, "dom_after": {"visibility": "visible"}}))
        self.assertTrue(s8.visible_throughout({"dom_loaded": {"visibility": "visible"}, "dom_after": {"visibility": "visible"}}))


if __name__ == "__main__":
    unittest.main()
