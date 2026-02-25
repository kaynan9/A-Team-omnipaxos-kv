# Why Reads Are Linearizable in OmniPaxos KV

## How reads work

In OmniPaxos KV, `KVCommand::Get` is not handled locally. It is appended to the
OmniPaxos consensus log in exactly the same way as `Put`, `Delete`, and `Cas`:

```rust
// server.rs — handle_client_messages
ClientMessage::Append(command_id, kv_command) => {
    let _ = self.append_to_log(from, command_id, kv_command);
}
```

The result is only sent back to the client inside `update_database_and_respond`,
which runs after the entry is *decided* — i.e., after a quorum of nodes has
acknowledged it and OmniPaxos has committed it to the log:

```rust
// server.rs — handle_decided_entries
let decided_entries = self.omnipaxos.read_decided_suffix(self.current_decided_idx)?;
// ...
self.update_database_and_respond(decided_commands);
```

## Why this is linearizable

A read issued by a client gets a sequence number (its position in the log) through
the normal Paxos prepare/accept/decide rounds. Every operation that was decided
*before* this position is visible to the read; every operation decided *after* is
not. This satisfies the definition of linearizability: each operation appears to
take effect atomically at a single point in real time that falls between its
invocation and its response, and that point is totally ordered with respect to all
other operations.

Concretely:

1. Client sends `Get(k)` to the coordinator.
2. The coordinator proposes it as a log entry.
3. A quorum accepts the entry; it is decided at index *i*.
4. All entries at indices 0 … *i-1* — including any concurrent `Put(k, v)` that
   was decided earlier — are applied to the state machine first.
5. The coordinator reads the resulting database value and replies.

The response therefore reflects the state of the system at a well-defined point in
the total order of the log, making the read linearizable.

## The alternative: local reads from followers

A common optimisation is to serve reads directly from a replica's local state
without going through consensus. This is *not* linearizable in general:

- A follower may lag behind the leader by one or more decided entries.
- A client could write a value via the leader (`Put(k, "new")`), then immediately
  read from a stale follower that has not yet applied that entry, receiving the old
  value.
- This violates linearizability because the read appears to go backwards in time
  relative to the write that preceded it.

Techniques such as read leases or Raft's `ReadIndex` can restore linearizability
for local reads, but they add protocol complexity. OmniPaxos KV avoids this
entirely by routing all reads through the log.

## Trade-off

Consensus reads are slower than local reads because they require a full Paxos
round-trip and quorum acknowledgement. For workloads that can tolerate stale reads
(eventual or session consistency), local reads would reduce latency and leader
load. OmniPaxos KV prioritises correctness: every read reflects the most recently
committed state, which makes the history produced by the test harness checkable by
a linearizability verifier such as Knossos.
