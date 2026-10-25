// One write at a time (CQ-33). A button disabled by `busy.on` is not enough on its own: a double click can
// land before the page redraws, so `run` also ignores a call made while one is in flight.

export class Busy {
  on = $state(false);

  /** Run `work` unless one is already running; the flag is cleared when it ends, however it ends. */
  async run(work: () => Promise<void>): Promise<void> {
    if (this.on) return;
    this.on = true;
    try {
      await work();
    } finally {
      this.on = false;
    }
  }
}
