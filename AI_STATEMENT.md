# AI statement

Much of this repository, including the research in [`plan.md`](plan.md), the specification, the code and the tests,
was written with Claude (Anthropic) under the direction of the maintainer, who is accountable for every change
([`MAINTAINERS.md`](MAINTAINERS.md)).

- **Research** was done by a model with web search. Facts that could not be confirmed against a primary source are
  marked `[uncertain]` in `plan.md` and listed in task AS-11. Treat the research as a starting point, not as advice.
- **Clinical content** (what to show, what to alert on, what a hazard is) has had **no clinical review**. No
  Clinical Safety Officer is appointed yet (open question OQ-2). Nothing here is clinically validated.
- **Translations** are machine-written and unreviewed.
- **No claim of compliance or certification** (HIPAA, ONC, DTAC, DSPT, DCB0129, MHRA) is made anywhere.

Tests are written alongside the code and run in CI; a passing test shows the behaviour the test describes, nothing
more.
