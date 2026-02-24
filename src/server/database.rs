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
