import importlib.util
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("chk", os.path.join(HERE, "check-hazard-log.py"))
chk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(chk)

ROOT = os.path.dirname(HERE)


def row(**over):
    base = dict(id="H-001", title="t", hazard="h", cause="c", effect="e", severity="3", likelihood="2",
                score="6", controls="x", evidence="AGENTS.md", residual="r", status="open",
                reviewed_by="", added="2026-10-21")
    base.update(over)
    return base


class HazardLog(unittest.TestCase):
    def check(self, *rows):
        return chk.problems(list(rows), chk.COLUMNS, ROOT)

    def test_the_real_log_passes(self):
        self.assertEqual(chk.main(["x", ROOT]), 0)

    def test_a_good_row_passes(self):
        self.assertEqual(self.check(row()), [])

    def test_a_wrong_score_is_caught(self):
        self.assertTrue(any("score must be" in p for p in self.check(row(score="7"))))

    def test_out_of_range_and_non_numbers_are_caught(self):
        self.assertTrue(self.check(row(severity="6", score="12")))
        self.assertTrue(self.check(row(severity="x")))

    def test_duplicate_ids_are_caught(self):
        self.assertTrue(any("duplicate" in p for p in self.check(row(), row())))

    def test_nobody_can_close_or_accept_without_a_named_reviewer(self):
        for status in ("accepted", "closed"):
            self.assertTrue(any("reviewed_by" in p for p in self.check(row(status=status))))
            self.assertEqual(self.check(row(status=status, reviewed_by="A. Person")), [])

    def test_controlled_needs_controls_and_unknown_status_is_refused(self):
        self.assertTrue(self.check(row(status="controlled-unreviewed", controls="")))
        self.assertTrue(self.check(row(status="mitigated")))

    def test_missing_evidence_files_are_caught(self):
        self.assertTrue(any("does not exist" in p for p in self.check(row(evidence="AGENTS.md;nope/none.rs"))))

    def test_wrong_columns_are_caught(self):
        self.assertTrue(chk.problems([], ["id"], ROOT))


if __name__ == "__main__":
    unittest.main()
