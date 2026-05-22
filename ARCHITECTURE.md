# signal-version-handover

`signal-version-handover` is the shared private-upgrade contract for component
version transitions.

## Boundary

This crate is a signal contract. It owns only the typed wire vocabulary. Runtime
state machines live in daemons or in sema-upgrade; schema projection logic lives
in `version-projection`; component-specific record transforms live in the
component runtime crate.

## Protocol Shape

The current prototype models six operations:

- `AskHandoverMarker` asks the current version for its schema and write marker.
- `ReadyToHandover` tells the current version that the next version can accept
  traffic from the recorded marker.
- `HandoverCompleted` confirms that active traffic has moved to the next
  version.
- `Mirror` forwards a write from one version to the other.
- `Divergence` records a write that cannot be represented in the other version.
- `RecoverFromFailure` starts a later reconciliation flow after a failed
  transition.

Accepted replies are intentionally small and typed. Rejection details stay in
`HandoverRejectionReason`.

## Non-Goals

This crate does not attempt to make cross-daemon writes magically atomic. It
provides the wire records that an atomic-or-compensating handover state machine
can use. The state machine must still decide when traffic flips, when old
writers freeze, and how divergence records are reconciled.
