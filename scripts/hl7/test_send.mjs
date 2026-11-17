// Tests for the HL7 sample sender (IH-11): node --test scripts/hl7/test_send.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:net";
import { mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { answer, fill, parse, run, VT, FS, CR } from "./send.mjs";

test("a file becomes one message: \\r between segments, placeholders filled", () => {
  assert.equal(
    fill("MSH|^~\\&|A|B|C|D|t||ADT^A28|{{control}}|P|2.5\nPID|1||{{mrn}}^^^LOCAL\n\n", { control: "C1", mrn: "M9" }),
    "MSH|^~\\&|A|B|C|D|t||ADT^A28|C1|P|2.5\rPID|1||M9^^^LOCAL",
  );
});

test("an answer is read from MSA", () => {
  assert.deepEqual(answer("MSH|^~\\&\rMSA|AR|C1|kept for review"), { code: "AR", text: "kept for review" });
  assert.deepEqual(answer("MSH|^~\\&"), { code: null, text: null });
});

test("options are checked", () => {
  assert.throws(() => parse([]), /at least one/);
  assert.throws(() => parse(["--port", "0", "a.hl7"]), /port/);
  assert.throws(() => parse(["--count", "x", "a.hl7"]), /count/);
  assert.throws(() => parse(["--bogus", "a.hl7"]), /unknown/);
  assert.equal(parse(["--tls", "--port", "7000", "a.hl7"]).port, 7000);
});

test("against a listener: each send has its own control id, one patient for the run", async () => {
  const seen = [];
  const server = createServer((socket) => {
    let buf = Buffer.alloc(0);
    socket.on("data", (chunk) => {
      buf = Buffer.concat([buf, chunk]);
      let end;
      while ((end = buf.indexOf(Buffer.from([FS, CR]))) >= 0) {
        const text = buf.subarray(buf.indexOf(VT) + 1, end).toString();
        buf = buf.subarray(end + 2);
        seen.push(text);
        const control = text.split("\r")[0].split("|")[9];
        const code = text.includes("ORU^") ? "AR" : "AA";
        socket.write(Buffer.concat([Buffer.from([VT]), Buffer.from(`MSH|^~\\&\rMSA|${code}|${control}|`), Buffer.from([FS, CR])]));
      }
    });
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const dir = mkdtempSync(join(tmpdir(), "hl7-send-"));
  writeFileSync(join(dir, "a.hl7"), "MSH|^~\\&|A|B|C|D|t||ADT^A28|{{control}}|P|2.5\nPID|1||{{mrn}}^^^LOCAL\n");
  writeFileSync(join(dir, "b.hl7"), "MSH|^~\\&|A|B|C|D|t||ORU^R01|{{control}}|P|2.5\n");
  try {
    const results = await run(
      { host: "127.0.0.1", port: server.address().port, count: 2, files: [join(dir, "a.hl7"), join(dir, "b.hl7")] },
      () => {},
    );
    assert.deepEqual(results.map((r) => `${r.file}:${r.round}:${r.code}`), ["a.hl7:1:AA", "b.hl7:1:AR", "a.hl7:2:AA", "b.hl7:2:AR"]);
    const controls = seen.map((m) => m.split("\r")[0].split("|")[9]);
    assert.equal(new Set(controls).size, 4, "a new control id per send");
    const mrns = seen.filter((m) => m.includes("PID")).map((m) => m.split("PID|1||")[1].split("^")[0]);
    assert.equal(new Set(mrns).size, 1, "one patient for the run");
  } finally {
    server.close();
  }
});
