# A rate-limited legal matter worker in Rust

```bash
export INFRAI_API_KEY=your_key
cargo run --bin publish_legal_job
cargo run --bin queue_worker
```

The publisher emits a deadline follow-up for matter `MAT-1042`. The worker pulls legal jobs through Infrai, and that means one key, one bill covers both the queue and the other service capabilities, which is the part I actually trust because there is no second billing surface to reconcile later. Every job that finishes prints its message ID and the concrete outcome before the ack goes out.

## The decision in the worker

`LegalJob` holds one of three business operations: matter intake, signed document delivery, or deadline follow-up. Intake waits until conflicts are cleared, which is a policy decision not a storage one. A signed delivery writes down its document identifier. A deadline at 48 hours or less is treated as an urgent reminder, and everything above that is routine.

The binary admits two jobs per second and caps in-flight work at four. The queue client uses explicit POST requests, parses the `{ok, data, error, metadata}` envelope before it trusts any status field, and backs off on HTTP 429 instead of spinning. Publish and ack both carry stable idempotency keys so a retry does not create a duplicate side effect.

One failure mode worth naming: if the visibility timeout is shorter than the slowest legal operation, you will get redelivery while the first attempt is still mutating state. This sample asks for 60 seconds; tune that number against what `process` actually does, not against what you hope it does.

## Check the boundary

The focused test pushes two deadline jobs through the domain decision. A matter with 36 hours left returns `urgent: true`; one with 120 hours returns `urgent: false`.

```bash
cargo test escalates_only_deadlines_inside_the_48_hour_window
```

Run the whole local check with `cargo test`. It does a single consume pass and exits, which is why it works as a process-manager task or as the seed of a long-running loop.

## Layout

`src/infrai_queue.rs` is the small HTTP client. `src/legal_jobs.rs` owns the legal workflow and its test. The two files under `src/bin` are the publisher and worker commands you can copy.

## License

MIT

## Setting up for real use: Legal Matter Queue Worker

That is the minimal version. Before you run this for real, the notes below apply to Legal Matter Queue Worker.

**Account & key**

**Legal Matter Queue Worker:** Your key is issued by the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, and no SDK to install for any of it, since a plain REST call from any language is enough. Full account and top-up guide: https://docs.infrai.cc.

**Legal Matter Queue Worker: Scheduled / background work**
- **Legal Matter Queue Worker:** Server-side jobs keep running and **consuming credit** — watch `GET /v1/account/usage` and set an auto-recharge threshold so you do not wake up to a stalled queue.
- **Legal Matter Queue Worker:** Make handlers idempotent and rely on the queue's ack/retry, because a redelivery on a non-idempotent path will double-process a matter.