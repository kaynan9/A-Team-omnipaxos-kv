use omnipaxos_kv::common::kv::KVCommand;
use std::collections::HashMap;

pub enum CommandResult {
    WriteOk,
    ReadOk(Option<String>),
    CasOk,
    CasFailed { current: Option<String> },
}

pub struct Database {
    db: HashMap<String, String>,
}

impl Database {
    pub fn new() -> Self {
        Self { db: HashMap::new() }
    }

    pub fn handle_command(&mut self, command: KVCommand) -> CommandResult {
        match command {
            KVCommand::Put(key, value) => {
                self.db.insert(key, value);
                CommandResult::WriteOk
            }
            KVCommand::Delete(key) => {
                self.db.remove(&key);
                CommandResult::WriteOk
            }
            KVCommand::Get(key) => {
                let value = self.db.get(&key).cloned();
                CommandResult::ReadOk(value)
            }
            KVCommand::Cas(key, expected, new_value) => {
                let current = self.db.get(&key).cloned();
                if current == expected {
                    self.db.insert(key, new_value);
                    CommandResult::CasOk
                } else {
                    CommandResult::CasFailed { current }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn put(db: &mut Database, key: &str, value: &str) -> CommandResult {
        db.handle_command(KVCommand::Put(key.to_string(), value.to_string()))
    }

    fn get(db: &mut Database, key: &str) -> CommandResult {
        db.handle_command(KVCommand::Get(key.to_string()))
    }

    fn cas(db: &mut Database, key: &str, expected: Option<&str>, new: &str) -> CommandResult {
        db.handle_command(KVCommand::Cas(
            key.to_string(),
            expected.map(str::to_string),
            new.to_string(),
        ))
    }

    fn assert_read_eq(result: CommandResult, expected: Option<&str>) {
        match result {
            CommandResult::ReadOk(v) => assert_eq!(v.as_deref(), expected),
            other => panic!("Expected ReadOk, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Basic linearizability — write then read returns written value
    // -----------------------------------------------------------------------

    #[test]
    fn write_then_read_returns_written_value() {
        // Linearizability requires: if a write completes, any subsequent read
        // must observe that write (or a later one). Since all operations are
        // serialized through the consensus log before reaching the database,
        // a Get that follows a Put in the decided log must see the Put's value.
        let mut db = Database::new();

        // Read before any write → None
        assert_read_eq(get(&mut db, "k1"), None);

        // Write, then read back
        assert!(matches!(put(&mut db, "k1", "v1"), CommandResult::WriteOk));
        assert_read_eq(get(&mut db, "k1"), Some("v1"));

        // Overwrite, then read back
        assert!(matches!(put(&mut db, "k1", "v2"), CommandResult::WriteOk));
        assert_read_eq(get(&mut db, "k1"), Some("v2"));
    }

    // -----------------------------------------------------------------------
    // Test 2: No stale reads — reads always reflect the latest committed write
    //
    // In OmniPaxos-KV, reads cannot be stale because KVCommand::Get is
    // appended to the consensus log (server.rs:append_to_log) and only
    // executed after it is decided (server.rs:update_database_and_respond).
    // There is no code path that reads directly from the Database without
    // going through consensus. This test verifies the state machine property
    // that underpins this guarantee: after a sequence of writes to the same
    // key, a read always returns the most recently written value — never an
    // older one.
    // -----------------------------------------------------------------------

    #[test]
    fn reads_always_reflect_latest_write_no_stale_data() {
        let mut db = Database::new();

        // Simulate a decided log: W(k1,v1) → W(k1,v2) → W(k1,v3) → R(k1)
        // In a system with stale follower reads, R might return v1 or v2.
        // In our system, R is serialized after W(v3) in the log, so it must
        // return v3.
        put(&mut db, "k1", "v1");
        put(&mut db, "k1", "v2");
        put(&mut db, "k1", "v3");
        assert_read_eq(get(&mut db, "k1"), Some("v3"));

        // Interleave reads between writes to different keys — each read must
        // see exactly the state at its position in the total order.
        put(&mut db, "k2", "a");
        assert_read_eq(get(&mut db, "k2"), Some("a"));
        put(&mut db, "k2", "b");
        assert_read_eq(get(&mut db, "k2"), Some("b"));

        // A read on an unwritten key must return None, not a phantom value
        assert_read_eq(get(&mut db, "k3"), None);
    }

    // -----------------------------------------------------------------------
    // Test 3: Concurrent writes — interleaved operations from multiple
    //         "processes" applied in decided order always yield consistent
    //         reads.
    //
    // In a real cluster, multiple clients submit operations concurrently.
    // OmniPaxos serializes them into a single decided log. This test
    // simulates that total order and verifies each read reflects the exact
    // state at its log position.
    // -----------------------------------------------------------------------

    #[test]
    fn concurrent_writes_reads_reflect_total_order() {
        let mut db = Database::new();

        // Simulate decided log from two concurrent clients (processes P0, P1):
        //   log[0]: P0 Put(k1, "p0-v1")
        //   log[1]: P1 Put(k1, "p1-v1")   ← overwrites P0's write
        //   log[2]: P0 Get(k1)             ← must see "p1-v1", not "p0-v1"
        //   log[3]: P1 Put(k1, "p1-v2")
        //   log[4]: P1 Get(k1)             ← must see "p1-v2"
        //   log[5]: P0 CAS(k1, "p1-v2", "p0-v2")  ← succeeds
        //   log[6]: P1 Get(k1)             ← must see "p0-v2"

        put(&mut db, "k1", "p0-v1");
        put(&mut db, "k1", "p1-v1");
        // P0's read must see P1's later write
        assert_read_eq(get(&mut db, "k1"), Some("p1-v1"));

        put(&mut db, "k1", "p1-v2");
        assert_read_eq(get(&mut db, "k1"), Some("p1-v2"));

        // CAS by P0 using the value P1 wrote
        assert!(matches!(
            cas(&mut db, "k1", Some("p1-v2"), "p0-v2"),
            CommandResult::CasOk
        ));
        // P1's subsequent read must see P0's CAS result
        assert_read_eq(get(&mut db, "k1"), Some("p0-v2"));
    }

    // -----------------------------------------------------------------------
    // Test 4: CAS linearizability — exactly-once semantics under contention
    //
    // Two processes racing to CAS the same key: exactly one must succeed and
    // the other must fail with the correct current value. Subsequent reads
    // must reflect the winning CAS.
    // -----------------------------------------------------------------------

    #[test]
    fn cas_exactly_once_under_contention() {
        let mut db = Database::new();

        // Setup: key exists with initial value
        put(&mut db, "k1", "init");

        // Two CAS operations decided in sequence — both expect "init":
        //   CAS(k1, "init", "winner")  → CasOk
        //   CAS(k1, "init", "loser")   → CasFailed { current: "winner" }
        assert!(matches!(
            cas(&mut db, "k1", Some("init"), "winner"),
            CommandResult::CasOk
        ));
        match cas(&mut db, "k1", Some("init"), "loser") {
            CommandResult::CasFailed { current } => {
                assert_eq!(current, Some("winner".to_string()));
            }
            other => panic!("Expected CasFailed, got {:?}", std::mem::discriminant(&other)),
        }

        // Read must reflect the winning CAS
        assert_read_eq(get(&mut db, "k1"), Some("winner"));
    }

    // -----------------------------------------------------------------------
    // Test 5: Delete followed by read returns None — no ghost reads
    // -----------------------------------------------------------------------

    #[test]
    fn delete_then_read_returns_none() {
        let mut db = Database::new();
        put(&mut db, "k1", "v1");
        assert_read_eq(get(&mut db, "k1"), Some("v1"));

        db.handle_command(KVCommand::Delete("k1".to_string()));
        assert_read_eq(get(&mut db, "k1"), None);

        // CAS with expected=None should now succeed (key absent)
        assert!(matches!(
            cas(&mut db, "k1", None, "v2"),
            CommandResult::CasOk
        ));
        assert_read_eq(get(&mut db, "k1"), Some("v2"));
    }
}
