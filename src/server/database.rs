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


    #[test]
    fn write_then_read_returns_written_value() {
        let mut db = Database::new();

        assert_read_eq(get(&mut db, "k1"), None);

        assert!(matches!(put(&mut db, "k1", "v1"), CommandResult::WriteOk));
        assert_read_eq(get(&mut db, "k1"), Some("v1"));

        assert!(matches!(put(&mut db, "k1", "v2"), CommandResult::WriteOk));
        assert_read_eq(get(&mut db, "k1"), Some("v2"));
    }


    #[test]
    fn reads_always_reflect_latest_write_no_stale_data() {
        let mut db = Database::new();

    
        put(&mut db, "k1", "v1");
        put(&mut db, "k1", "v2");
        put(&mut db, "k1", "v3");
        assert_read_eq(get(&mut db, "k1"), Some("v3"));

        put(&mut db, "k2", "a");
        assert_read_eq(get(&mut db, "k2"), Some("a"));
        put(&mut db, "k2", "b");
        assert_read_eq(get(&mut db, "k2"), Some("b"));

        assert_read_eq(get(&mut db, "k3"), None);
    }


    #[test]
    fn concurrent_writes_reads_reflect_total_order() {
        let mut db = Database::new();

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



    #[test]
    fn cas_exactly_once_under_contention() {
        let mut db = Database::new();

        put(&mut db, "k1", "init");

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

        assert_read_eq(get(&mut db, "k1"), Some("winner"));
    }



    #[test]
    fn delete_then_read_returns_none() {
        let mut db = Database::new();
        put(&mut db, "k1", "v1");
        assert_read_eq(get(&mut db, "k1"), Some("v1"));

        db.handle_command(KVCommand::Delete("k1".to_string()));
        assert_read_eq(get(&mut db, "k1"), None);

        assert!(matches!(
            cas(&mut db, "k1", None, "v2"),
            CommandResult::CasOk
        ));
        assert_read_eq(get(&mut db, "k1"), Some("v2"));
    }
}
