use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum EventType {
    Invoke,
    Ok,
    Fail,
    #[allow(dead_code)] // produced by Knossos internally for dangling invokes; kept for write_edn completeness
    Info,
}

#[derive(Debug, Clone)]
pub enum FunctionType {
    Read,
    Write,
    Cas,
}

#[derive(Debug, Clone)]
pub struct HistoryEvent {
    pub process: usize,
    pub event_type: EventType,
    pub function: FunctionType,
    pub key: String,
    pub value: Option<String>,
    pub expected: Option<String>,
}

#[derive(Clone)]
pub struct History {
    events: Arc<Mutex<Vec<HistoryEvent>>>,
}

impl History {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, event: HistoryEvent) {
        self.events.lock().unwrap().push(event);
    }

    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Returns (invoke, ok, fail, info) counts.
    pub fn type_counts(&self) -> (usize, usize, usize, usize) {
        let events = self.events.lock().unwrap();
        let mut invoke = 0;
        let mut ok = 0;
        let mut fail = 0;
        let mut info = 0;
        for e in events.iter() {
            match e.event_type {
                EventType::Invoke => invoke += 1,
                EventType::Ok     => ok += 1,
                EventType::Fail   => fail += 1,
                EventType::Info   => info += 1,
            }
        }
        (invoke, ok, fail, info)
    }

    pub fn write_edn(&self, path: &str) -> std::io::Result<()> {
        let events = self.events.lock().unwrap();
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);

        writeln!(w, "[")?;
        for e in events.iter() {
            let type_kw = match e.event_type {
                EventType::Invoke => ":invoke",
                EventType::Ok    => ":ok",
                EventType::Fail  => ":fail",
                EventType::Info  => ":info",
            };
            let f_kw = match e.function {
                FunctionType::Read  => ":read",
                FunctionType::Write => ":write",
                FunctionType::Cas   => ":cas",
            };

            let value_edn = match e.function {
                FunctionType::Read => {
                    // :value ["key" nil-or-"val"]
                    let v = edn_str(e.value.as_deref());
                    format!("[{} {}]", edn_str(Some(&e.key)), v)
                }
                FunctionType::Write => {
                    // :value ["key" "val"]
                    let v = edn_str(e.value.as_deref());
                    format!("[{} {}]", edn_str(Some(&e.key)), v)
                }
                FunctionType::Cas => {
                    // :value ["key" expected new_value]
                    let exp = edn_str(e.expected.as_deref());
                    let new = edn_str(e.value.as_deref());
                    format!("[{} {} {}]", edn_str(Some(&e.key)), exp, new)
                }
            };

            writeln!(
                w,
                " {{:process {} :type {} :f {} :value {}}}",
                e.process, type_kw, f_kw, value_edn
            )?;
        }
        writeln!(w, "]")?;
        Ok(())
    }
}

fn edn_str(s: Option<&str>) -> String {
    match s {
        Some(v) => format!("\"{}\"", v),
        None    => "nil".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(
        process: usize,
        event_type: EventType,
        function: FunctionType,
        key: &str,
        value: Option<&str>,
        expected: Option<&str>,
    ) -> HistoryEvent {
        HistoryEvent {
            process,
            event_type,
            function,
            key: key.to_string(),
            value: value.map(str::to_string),
            expected: expected.map(str::to_string),
        }
    }

    #[test]
    fn record_and_len() {
        let h = History::new();
        assert_eq!(h.len(), 0);
        h.record(make_event(0, EventType::Invoke, FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(0, EventType::Ok,     FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(1, EventType::Invoke, FunctionType::Read,  "k1", None,       None));
        h.record(make_event(1, EventType::Ok,     FunctionType::Read,  "k1", Some("v0"), None));
        assert_eq!(h.len(), 4);
    }

    #[test]
    fn type_counts_correct() {
        let h = History::new();
        h.record(make_event(0, EventType::Invoke, FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(0, EventType::Ok,     FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(1, EventType::Invoke, FunctionType::Read,  "k1", None,       None));
        h.record(make_event(1, EventType::Ok,     FunctionType::Read,  "k1", Some("v0"), None));

        let (invokes, oks, fails, infos) = h.type_counts();
        assert_eq!(invokes, 2);
        assert_eq!(oks,     2);
        assert_eq!(fails,   0);
        assert_eq!(infos,   0);
    }

    #[test]
    fn write_edn_format() {
        let h = History::new();
        h.record(make_event(0, EventType::Invoke, FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(0, EventType::Ok,     FunctionType::Write, "k1", Some("v0"), None));
        h.record(make_event(1, EventType::Invoke, FunctionType::Read,  "k1", None,       None));
        h.record(make_event(1, EventType::Ok,     FunctionType::Read,  "k1", Some("v0"), None));

        let path = "test_history_output.edn";
        h.write_edn(path).expect("write_edn failed");
        let contents = std::fs::read_to_string(path).expect("could not read output file");
        std::fs::remove_file(path).ok();

        // Outer brackets.
        assert!(contents.starts_with('['), "should start with [");
        assert!(contents.trim_end().ends_with(']'), "should end with ]");

        // Every line between the brackets must be a valid EDN map line.
        let lines: Vec<&str> = contents
            .lines()
            .filter(|l| l.trim_start().starts_with('{'))
            .collect();
        assert_eq!(lines.len(), 4, "expected 4 event lines, got {}", lines.len());

        // Spot-check specific fields.
        assert!(lines[0].contains(":type :invoke"), "line 0 should be :invoke");
        assert!(lines[0].contains(":f :write"),     "line 0 should be :write");
        assert!(lines[0].contains(r#""k1""#),       "line 0 should contain key k1");
        assert!(lines[0].contains(r#""v0""#),       "line 0 should contain value v0");

        assert!(lines[1].contains(":type :ok"),     "line 1 should be :ok");
        assert!(lines[1].contains(":f :write"),     "line 1 should be :write");

        assert!(lines[2].contains(":type :invoke"), "line 2 should be :invoke");
        assert!(lines[2].contains(":f :read"),      "line 2 should be :read");
        assert!(lines[2].contains("nil"),           "line 2 invoke read value should be nil");

        assert!(lines[3].contains(":type :ok"),     "line 3 should be :ok");
        assert!(lines[3].contains(":f :read"),      "line 3 should be :read");
        assert!(lines[3].contains(r#""v0""#),       "line 3 ok read should echo server value");
    }
}
