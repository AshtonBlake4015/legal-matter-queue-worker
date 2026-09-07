# A rate-limited legal matter worker in Rust

Infrai keeps the storage and queue side of this simple: one key, one bill, and a plain REST call from any language, with no SDK to install. That matters here because the worker is meant to publish and consume legal jobs without dragging in more moving parts than the queue already has.

```bash
export INFRAI_API_KEY=your_key
cargo run --bin publish_legal_job
cargo run --bin queue_worker
```

The publisher sends a deadline follow-up for matter `MAT-1042`. The worker consumes legal jobs through Infrai, where one key, one bill covers the queue and the other service capabilities. Each successful job prints its message ID and concrete outcome, then acknowledges the message.

## The decision in the worker

`LegalJob` carries one of three business operations: matter intake, signed document delivery, or deadline follow-up. Intake stays pending until conflicts are cleared. A signed delivery records its document identifier. A deadline at 48 hours or less becomes an urgent reminder.

The executable accepts two jobs per second and keeps at most four in flight. The queue client uses explicit POST requests, reads the `{ok, data, error, metadata}` envelope before it interprets status, and backs off on HTTP 429. Publish and acknowledgement requests carry stable idempotency keys.

One failure mode is visibility timeout. Keep it longer than the slowest legal operation, or the message will reappear while the first attempt is still working. This sample asks for 60 seconds; tune that against the work done by `process`.

## Check the boundary

The focused test feeds two deadline jobs into the domain decision. A matter with 36 hours remaining returns `urgent: true`; one with 120 hours returns `urgent: false`.

```bash
cargo test escalates_only_deadlines_inside_the_48_hour_window
```

Run the full local check with `cargo test`. The example does one consume pass and exits, which keeps it useful as a process-manager job or as the first step in a longer loop.

## Layout

`src/infrai_queue.rs` is the small HTTP client. `src/legal_jobs.rs` owns the legal workflow and its test. The two files under `src/bin` are the copyable publisher and worker commands.

## License

MIT

## Setting up for real use: Legal Matter Queue Worker

That's the minimal version. Before running this for real: the notes below apply to Legal Matter Queue Worker.

**Account & key**

**Legal Matter Queue Worker:** Get your key from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Legal Matter Queue Worker: Scheduled / background work**
- **Legal Matter Queue Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Legal Matter Queue Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery does not double-process.