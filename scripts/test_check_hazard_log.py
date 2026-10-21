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

    def test_added_must_be_a_real_iso_date(self):
        for bad in ("21/10/2026", "2026-13-01", "2026-02-30", "yesterday"):
            self.assertTrue(any("added" in p for p in self.check(row(added=bad))), bad)
        self.assertEqual(self.check(row(added="2024-02-29")), [])


class Transitions(unittest.TestCase):
    """CQ-21: only a named reviewer may accept or close a hazard, judged against the base revision."""

    def test_unchanged_and_ordinary_edits_pass(self):
        base = [row()]
        self.assertEqual(chk.transitions(base, [row(controls="more")], "anyone", set()), [])
        self.assertEqual(chk.transitions(base, [row(), row(id="H-002")], "anyone", set()), [])

    def test_accepting_or_closing_needs_an_allowed_author(self):
        for status in ("accepted", "closed"):
            head = [row(status=status, reviewed_by="A. Person")]
            self.assertTrue(chk.transitions([row()], head, "agent", set()), status)
            self.assertTrue(chk.transitions([row()], head, "agent", {"cso"}), status)
            self.assertEqual(chk.transitions([row()], head, "cso", {"cso"}), [], status)

    def test_a_new_hazard_cannot_arrive_accepted(self):
        head = [row(), row(id="H-002", status="closed", reviewed_by="A. Person")]
        self.assertTrue(chk.transitions([row()], head, "agent", set()))

    def test_changing_the_reviewer_needs_an_allowed_author(self):
        base = [row(status="accepted", reviewed_by="A. Person")]
        head = [row(status="accepted", reviewed_by="Someone Else")]
        self.assertTrue(chk.transitions(base, head, "agent", set()))

    def test_reopening_is_allowed_to_anyone(self):
        base = [row(status="closed", reviewed_by="A. Person")]
        self.assertEqual(chk.transitions(base, [row(status="open", reviewed_by="A. Person")], "agent", set()), [])

    def test_removing_a_hazard_is_refused(self):
        self.assertTrue(chk.transitions([row(), row(id="H-002")], [row()], "agent", {"agent"}))

    def test_the_allow_list_ignores_comments_and_blanks(self):
        self.assertEqual(chk.parse_reviewers("# the CSO\n\n  cso-login  \n"), {"cso-login"})


if __name__ == "__main__":
    unittest.main()
