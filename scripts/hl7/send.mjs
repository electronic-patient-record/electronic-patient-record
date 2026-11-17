#!/usr/bin/env node
// Send HL7 v2 sample messages to an MLLP listener and print each answer (IH-11). For a test deployment only: it sends
// what the files hold, and the samples are synthetic (operations/hl7/samples).
//
//   node scripts/hl7/send.mjs [--host 127.0.0.1] [--port 2575] [--count 1] [--mrn M-1]
//                             [--tls] [--ca ca.pem] [--cert client.pem] [--key client.key] [--servername name]
//                             file.hl7 ...
//
// Each file is one message, a segment a line. `{{control}}` becomes a new control id on every send, so a resend is
// not a repeat; `{{mrn}}` becomes --mrn, or a new number once per run, so the files describe one patient. --count sends
// the whole set that many times. Exit status: 0 when every answer is AA, 1 when any is AE or AR, 2 on a usage or
// connection error. No dependency: Node's net and tls.
import { connect as tcp } from "node:net";
import { connect as tlsConnect } from "node:tls";
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { pathToFileURL } from "node:url";

export const VT = 0x0b;
export const FS = 0x1c;
export const CR = 0x0d;

/** A file's text as a message: line breaks as `\r`, the placeholders filled. */
export function fill(text, { control, mrn }) {
  return text
    .trimEnd()
    .split(/\r\n|\n|\r/)
    .join("\r")
    .replaceAll("{{control}}", control)
    .replaceAll("{{mrn}}", mrn);
}

/** MSA-1 and MSA-3 of an acknowledgement, or nulls when it has no MSA. */
export function answer(ack) {
  const msa = ack.split("\r").find((s) => s.startsWith("MSA|"));
  if (!msa) return { code: null, text: null };
  const f = msa.split("|");
  return { code: f[1] ?? null, text: f[3] ?? "" };
}

/** Open a connection; resolves to a function that sends one message and resolves to its acknowledgement text. */
export function open(opts) {
  return new Promise((resolve, reject) => {
    const socket = opts.tls
      ? tlsConnect({
          host: opts.host,
          port: opts.port,
          servername: opts.servername ?? opts.host,
          ca: opts.ca ? readFileSync(opts.ca) : undefined,
          cert: opts.cert ? readFileSync(opts.cert) : undefined,
          key: opts.key ? readFileSync(opts.key) : undefined,
        })
      : tcp({ host: opts.host, port: opts.port });
    let buffer = Buffer.alloc(0);
    let waiting = null;
    socket.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, chunk]);
      const end = buffer.indexOf(Buffer.from([FS, CR]));
      if (end >= 0 && waiting) {
        const start = buffer.indexOf(VT);
        const text = buffer.subarray(start + 1, end).toString("utf8");
        buffer = buffer.subarray(end + 2);
        const w = waiting;
        waiting = null;
        w.resolve(text);
      }
    });
    const fail = (e) => {
      if (waiting) waiting.reject(e);
      else reject(e);
    };
    socket.on("error", fail);
    socket.on("close", () => fail(new Error("the listener closed the connection")));
    socket.on(opts.tls ? "secureConnect" : "connect", () => {
      const send = (message) =>
        new Promise((res, rej) => {
          waiting = { resolve: res, reject: rej };
          socket.write(Buffer.concat([Buffer.from([VT]), Buffer.from(message, "utf8"), Buffer.from([FS, CR])]));
        });
      send.close = () => socket.end();
      resolve(send);
    });
  });
}

function parse(argv) {
  const opts = { host: "127.0.0.1", port: 2575, count: 1, tls: false, files: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const value = () => {
      const v = argv[++i];
      if (v === undefined) throw new Error(`${a} needs a value`);
      return v;
    };
    if (a === "--host") opts.host = value();
    else if (a === "--port") opts.port = Number(value());
    else if (a === "--count") opts.count = Number(value());
    else if (a === "--mrn") opts.mrn = value();
    else if (a === "--tls") opts.tls = true;
    else if (a === "--ca") opts.ca = value();
    else if (a === "--cert") opts.cert = value();
    else if (a === "--key") opts.key = value();
    else if (a === "--servername") opts.servername = value();
    else if (a.startsWith("--")) throw new Error(`unknown option ${a}`);
    else opts.files.push(a);
  }
  if (!opts.files.length) throw new Error("name at least one .hl7 file");
  if (!Number.isInteger(opts.port) || opts.port < 1 || opts.port > 65535) throw new Error("--port must be 1 to 65535");
  if (!Number.isInteger(opts.count) || opts.count < 1 || opts.count > 10000) throw new Error("--count must be 1 to 10000");
  return opts;
}

/** Send every file `count` times on one connection; resolves to `[{file, round, code, text}]`. */
export async function run(opts, log = console.log) {
  const mrn = opts.mrn ?? `M-${Date.now().toString(36).toUpperCase()}`;
  const texts = opts.files.map((f) => [basename(f), readFileSync(f, "utf8")]);
  const send = await open(opts);
  const results = [];
  try {
    let n = 0;
    for (let round = 1; round <= opts.count; round++) {
      for (const [file, text] of texts) {
        n += 1;
        const control = `S${Date.now().toString(36).toUpperCase()}${n}`;
        const ack = await send(fill(text, { control, mrn }));
        const { code, text: why } = answer(ack);
        results.push({ file, round, code, text: why });
        log(`${file}\t${round}\t${code ?? "?"}\t${why ?? ""}`);
      }
    }
  } finally {
    send.close();
  }
  return results;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  let opts;
  try {
    opts = parse(process.argv.slice(2));
  } catch (e) {
    console.error(`send: ${e.message}`);
    process.exit(2);
  }
  try {
    const results = await run(opts);
    process.exit(results.every((r) => r.code === "AA") ? 0 : 1);
  } catch (e) {
    console.error(`send: ${e.message}`);
    process.exit(2);
  }
}

export { parse };
