"""Tests for scripts/changelog.py (python3 -m unittest discover -s scripts)."""
import tempfile
import unittest
from pathlib import Path

import changelog

HEAD = "# Changelog\n\nintro\n\n## [Unreleased]\n\n## [0.6.0] — 2026-11-03\n\nold\n"


def frag(**files):
    root = Path(tempfile.mkdtemp())
    (root / "changes").mkdir()
    for name, text in files.items():
        (root / "changes" / name.replace("__", ".")).write_text(text, encoding="utf-8")
    (root / "CHANGELOG.md").write_text(HEAD, encoding="utf-8")
    return root


class Fragments(unittest.TestCase):
    def test_groups_in_order_and_places_the_release_below_unreleased(self):
        root = frag(a__fixed__md="Fixed a thing.", b__added__md="Added a thing.\nSecond line.", c__output__md="Output moved.")
        self.assertEqual(changelog.main(["release", "0.7.0", "2026-12-01"], root), 0)
        text = (root / "CHANGELOG.md").read_text(encoding="utf-8")
        self.assertLess(text.index("### Added"), text.index("### Output changes"))
        self.assertLess(text.index("### Output changes"), text.index("### Fixed"))
        self.assertIn("- Added a thing.\n  Second line.", text)
        self.assertLess(text.index("## [Unreleased]"), text.index("## [0.7.0] — 2026-12-01"))
        self.assertLess(text.index("## [0.7.0]"), text.index("## [0.6.0]"))
        self.assertEqual(list((root / "changes").glob("*.md")), [])

    def test_refuses_an_existing_version(self):
        root = frag(a__fixed__md="x")
        with self.assertRaises(ValueError):
            changelog.release(HEAD, "0.6.0", "2026-12-01", changelog.fragments(root / "changes"))

    def test_validation(self):
        root = frag(a__nonsense__md="x", b__fixed__md="- leading bullet", c__fixed__md="", d__added__md="## heading")
        found = changelog.problems(changelog.fragments(root / "changes"))
        self.assertEqual(len(found), 4, found)
        self.assertEqual(changelog.main(["check"], root), 1)

    def test_check_and_empty(self):
        self.assertEqual(changelog.main(["check"], frag(a__fixed__md="ok")), 0)
        self.assertEqual(changelog.main(["preview"], frag()), 1)
        self.assertEqual(changelog.main(["release", "0.7.0"], frag()), 1)
        self.assertEqual(changelog.main(["release", "seven"], frag(a__fixed__md="x")), 2)


if __name__ == "__main__":
    unittest.main()
