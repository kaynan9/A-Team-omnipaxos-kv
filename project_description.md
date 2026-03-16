Battle-testing OmniPaxos with Jepsen [2-3 people per team]
1. Introduction

Distributed consensus algorithms promise strong consistency guarantees, but implementation details often introduce subtle bugs that only manifest under specific failure modes. While formal proofs verify the algorithm, they do not verify the code. The industry standard for validating distributed databases is Jepsen, a framework for randomized fault injection and black-box testing.

The key insight of Jepsen is that by subjecting a system to "nemeses"—simulated network partitions, process crashes, and clock skew—while concurrently executing operations, one can empirically verify if the system maintains its consistency guarantees (e.g., Linearizability).

In this project, you will explore the hypothesis: "The OmniPaxos implementation preserves linearizability under aggressive network partitioning and node failures."

You will extend the OmniPaxos Key-Value (KV) store with an API suitable for automated testing, implement a Jepsen (or Maelstrom) test suite that models client operations, and subject the cluster to network partitions to verify safety properties.

You will be provided the OmniPaxos KV repository with networking infrastructure. Your task is to build a "shim" that exposes the KV store to the test harness, implement a generator for random operations (reads, writes, CAS), and conduct experiments that attempt to break the consensus (e.g., via split-brain scenarios) to verify its resilience.
2. Requirements
2.1 Basic Requirements (30 points)

    Implement a Testable Shim: Modify the OmniPaxos KV example to expose a programmable API (HTTP, standard input/output, or simple TCP) that allows external agents to trigger operations programmatically.

    The shim must support put, get, and optionally cas (compare-and-swap) operations.

    Implement the Client & Generator: Create a test client (using Jepsen/Clojure or Maelstrom/Rust) that bridges the test generator to your Rust shim.

    The client must correctly interpret "indeterminate" states (e.g., when a request times out during a network partition, the operation might or might not have succeeded).
    Implement a generator that produces a mix of read and write operations.

    Fault Injection (The Nemesis): Configure a "Nemesis" that introduces failures during test execution:

    Partitioning: Isolate the leader from followers or split the cluster into two halves.
    Crashes: Kill and restart nodes randomly.

    Verification of Linearizability: Run the test suite under faults and use a linearizability checker (like Knossos) to analyze the operation history.

    Demonstrate that the history is linearizable (no stale reads or lost writes that were acknowledged).
    If a violation is found, analyze the logs to explain the bug.
    Hint: Note that simple local reads are generally not linearizable out of the box in consensus systems. What mechanisms can be implemented to ensure linearizable reads? Implement a solution to make linearizable reads safe, test it, and discuss your design in the report.

3. Project Information

    Category: Distributed Systems Verification, Fault Tolerance, Testing
    Language and libraries: Rust, Clojure (optional if using Jepsen), Maelstrom (optional), Docker
    Repository: https://github.com/haraldng/omnipaxos-kv 

4. Key Learning Outcomes

    Understanding the difference between theoretical correctness and implementation safety.
    Practical experience with black-box testing and fault injection techniques.
    Analyzing consistency models (Linearizability vs. Sequential Consistency) using history checkers.
    Debugging distributed state machines under partial network failure.

5. References and Background

    Jepsen.io: https://jepsen.io 
    Maelstrom: https://github.com/jepsen-io/maelstrom  - A workbench for learning distributed systems.
    Knossos: The linearizability checker used to verify operation histories.
    OmniPaxos documentation: For understanding expected behavior during leader changes.

