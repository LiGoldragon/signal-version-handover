# INTENT — signal-version-handover

*The private daemon-to-daemon wire contract carrying the handover
protocol between two versions of one component. Defines the typed
vocabulary two sibling daemons run to hand the public surface from one
version to its next version without losing writes. Companion to
`ARCHITECTURE.md` and `Cargo.toml`. Maintenance:
`primary/skills/repo-intent.md`.*

## Repo-scope only

This file carries only the intent that is FOR this
`signal-version-handover` contract. Workspace-shape intent stays in the
primary workspace `primary/INTENT.md`. Meta administrative
authority intent (force-flip / rollback / quarantine) stays in
`meta-signal-version-handover/INTENT.md`.

## Why this repo exists

`signal-version-handover` is a **signal contract crate** for the
private, daemon-to-daemon upgrade boundary. It owns the typed wire
vocabulary for the protocol two sibling daemons run — current version ↔
next version — to migrate the public surface across a schema change
without dropping writes. The wire is carried over a per-daemon private
upgrade socket (one socket per version, mode `0600`, version suffix in
the path; both daemons run under the same persona-owner UID). The crate
is pure wire vocabulary — no daemon code, no runtime state machine, no
migration logic.

## The channel shape

The `VersionHandover` channel carries six operations and seven replies:

- **Operations** (next → current, except recovery either-way):
  `AskHandoverMarker` (next asks current for schema hash + commit
  sequence + last record id), `ReadyToHandover` (next has copied state
  up to a marker), `HandoverCompleted` (public traffic has moved;
  current closes its sockets), `Mirror` (next forwards a write back to
  current), `Divergence` (next records a write reverse-projection cannot
  represent), `RecoverFromFailure` (reconciliation after a failed
  transition).
- **Replies:** `HandoverMarkerReported`, `HandoverAccepted`,
  `HandoverFinalized`, `Mirrored`, `DivergenceRecorded`, `Recovered`,
  `HandoverRejected` (with typed `HandoverRejectionReason`).

`HandoverMarker` carries the load-bearing durability shape —
`ContractVersion` schema hash, sema-engine `commit_sequence`, write
counter, last record id, and daemon-stamped capture date/time.

## The mirror-payload discipline

`MirrorPayload` carries an **unspecified raw payload** — raw bytes plus
a `RecordKind` discriminant — so the wire stays version-pair-blind. The
load-bearing receiver-side rule: raw payload bytes MUST land in a
SEPARATE container outside the receiver's typed database. The typed
database only ever accepts records already reverse-projected through
`version-projection` into the receiver's own shape. A non-representable
payload becomes a typed `Divergence` operation on this wire — never a
silent drop and never a raw row in the typed database. This keeps two
invariants together: the contract stays version-pair-blind, and the
typed database stays clean.

## Channels are closed, boundaries are named

- Wire enums are closed. No `Unknown` escape hatch.
- The contract is version-pair-blind: no per-pair signal-X variants leak
  in; receivers decode raw mirror bytes through the appropriate
  versioned library.
- Rejection detail is the typed `HandoverRejectionReason` closed enum,
  not free text.
- Identity, commit sequence, write counter, and capture date/time on the
  marker are daemon-minted, not caller-supplied.

## Constraints

- This crate carries only typed wire vocabulary and round-trip
  witnesses.
- No runtime code: no daemon binary, no socket binding, no runtime state
  machine, no migration logic.
- The crate does not own administrative authority verbs (force-flip /
  rollback / quarantine) — those live in
  `meta-signal-version-handover`.
- The crate does not depend on `version-projection` at the trait level;
  daemons that compose the two import both.
- The crate does not depend on any `signal-persona-*` contract.
- Every operation and reply round-trips through both rkyv and NOTA;
  witnesses live in `tests/`.
- Receiver-side storage discipline: raw bytes land in a SEPARATE
  container outside the typed database. The contract does not own the
  container's on-disk shape, but it requires the separation.

## Non-ownership

This crate does not own:

- atomic-write coordination across daemons (the state machine lives in
  component daemons or in `sema-upgrade`'s handover prototype);
- the traffic-flip / selector-flip decision (the orchestrator's
  responsibility);
- write-freeze enforcement (the current daemon enters HandoverMode from
  its own state, not a wire flag);
- divergence reconciliation policy (recorded divergences land in
  introspect; reconciliation is downstream tooling);
- per-version record transforms (the component runtime's per-version
  migration module);
- schema projection logic (`version-projection`).

## See also

- `ARCHITECTURE.md` — protocol sequence, the marker durability shape,
  the mirror-payload container discipline, and boundary diagram.
- `../meta-signal-version-handover/ARCHITECTURE.md` — meta
  administrative authority sibling.
- `../version-projection/ARCHITECTURE.md` — projection library for
  reverse-projecting mirror bytes.
- `../sema-upgrade/ARCHITECTURE.md` — handover prototype state machine
  driven by this contract.
- `primary/skills/contract-repo.md` — contract repo discipline and
  naming rules.
- `primary/skills/component-triad.md` — repo triad structure and wire
  layers.
