use super::{init::enable_console_log, log_entry::LogEntry};

#[test]
fn storage_events() {
    enable_console_log();

    LogEntry::ReadFromFile {
        time: 10.15,
        node: "node".to_owned(),
        request_id: 14,
        file_name: "file1.txt".to_owned(),
        bytes: 100,
    }
    .print();

    LogEntry::ReadRequestSucceed {
        time: 12.0,
        node: "node".to_owned(),
        request_id: 14,
        file_name: "file1.txt".to_owned(),
        bytes: 100,
    }
    .print();

    LogEntry::WriteToFile {
        time: 15.30,
        node: "node".to_owned(),
        request_id: 15,
        file_name: "file2.txt".to_owned(),
        bytes: 256,
    }
    .print();

    LogEntry::WriteRequestFailed {
        time: 16.10,
        node: "node".to_owned(),
        request_id: 15,
        file_name: "file2.txt".to_owned(),
        reason: "storage unavailable".to_owned(),
        bytes: 256,
    }
    .print();
}

#[test]
fn node_lifecycle() {
    enable_console_log();

    LogEntry::NodeCrashed {
        time: 10.15,
        node: "crashed".to_owned(),
    }
    .print();

    LogEntry::NodeRecovered {
        time: 10.25,
        node: "crashed".to_owned(),
    }
    .print();

    LogEntry::NodeShutdown {
        time: 10.35,
        node: "shut-downed".to_owned(),
    }
    .print();

    LogEntry::NodeReran {
        time: 10.45,
        node: "shut-downed".to_owned(),
    }
    .print();
}
