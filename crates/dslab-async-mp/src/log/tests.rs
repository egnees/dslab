use crate::log::log_entry::LogEntry;

use super::logger::Logger;

#[test]
fn logger_works() {
    let mut logger = Logger::default();
    assert!(!logger.has_log_file());

    logger.log(LogEntry::NodeCrashed {
        time: 12.5,
        node: "crashed".to_owned(),
    });

    logger.log(LogEntry::NodeRecovered {
        time: 15.5,
        node: "crashed".to_owned(),
    });

    assert_eq!(
        *logger.trace(),
        vec![
            LogEntry::NodeCrashed {
                time: 12.5,
                node: "crashed".to_owned()
            },
            LogEntry::NodeRecovered {
                time: 15.5,
                node: "crashed".to_owned()
            }
        ]
    );
}
