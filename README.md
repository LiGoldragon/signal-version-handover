# signal-version-handover

`signal-version-handover` is the private upgrade signal contract used by two
versions of the same Persona component during a live handover.

The contract is deliberately narrow. It carries marker, readiness, completion,
mirror, divergence, and recovery messages. It does not own database migration
logic, socket selection, daemon state, or system deployment.
