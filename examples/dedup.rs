use flexi_logger::{
    filter::{LogLineFilter, LogLineWriter},
    DeferredNow,
};
use log::Record;
use std::{cmp::Ordering, num::NonZeroUsize, sync::Mutex};

fn main() {
    #[cfg(feature = "colors")]
    let format = flexi_logger::colored_detailed_format;
    #[cfg(not(feature = "colors"))]
    let format = flexi_logger::detailed_format;

    flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .format(format)
        .log_to_stdout()
        .filter(Box::new(DedupWriter::with_leeway(
            std::num::NonZeroUsize::new(2).unwrap(),
        )))
        .start()
        .unwrap();

    for i in 0..10 {
        log::info!("{}", if i == 5 { "bar" } else { "foo" });
    }

    log::info!("the end");
}

/// A helper to skip duplicated consecutive log lines.
pub struct DedupWriter {
    deduper: Mutex<Deduper>,
}
impl DedupWriter {
    /// Constructs a new [`Deduper`] that will skip duplicated entries after
    /// some record has been received for the consecutive times specified by
    /// `leeway`.
    pub fn with_leeway(leeway: NonZeroUsize) -> Self {
        Self {
            deduper: Mutex::new(Deduper::with_leeway(leeway)),
        }
    }
}
impl LogLineFilter for DedupWriter {
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()> {
        let mut deduper = self.deduper.lock().unwrap();
        let dedup_action = deduper.dedup(record);
        match dedup_action {
            DedupAction::Allow => {
                // Just log
                log_line_writer.write(now, record)
            }
            DedupAction::AllowLastOfLeeway(_) => {
                // Log duplicate
                log_line_writer.write(now, record)?;
                // Log warning
                log_line_writer.write(
                    now,
                    &log::Record::builder()
                        .level(log::Level::Warn)
                        .file_static(Some(file!()))
                        .line(Some(line!()))
                        .module_path_static(Some("flexi_logger"))
                        .target("flexi_logger")
                        .args(format_args!(
                            "last record has been repeated consecutive times, \
                             following duplicates will be skipped...",
                        ))
                        .build(),
                )
            }
            DedupAction::AllowAfterSkipped(skipped) => {
                // Log summary of skipped
                log_line_writer.write(
                    now,
                    &log::Record::builder()
                        .level(log::Level::Info)
                        .file_static(Some(file!()))
                        .line(Some(line!()))
                        .module_path_static(Some("flexi_logger"))
                        .target("flexi_logger")
                        .args(format_args!("last record was skipped {skipped} times"))
                        .build(),
                )?;
                // Log new record
                log_line_writer.write(now, record)
            }
            DedupAction::Skip => Ok(()),
        }
    }
}

// A helper to track duplicated consecutive logs and skip them until a
// different event is received.
struct Deduper {
    leeway: NonZeroUsize,
    last_record: LastRecord,
    duplicates: usize,
}

/// Action to be performed for some record.
#[derive(Debug, PartialEq, Eq)]
enum DedupAction {
    /// The record should be allowed and logged normally.
    Allow,
    /// The record is the last consecutive duplicate to be allowed.
    ///
    /// Any following duplicates will be skipped until a different event is
    /// received (or the duplicates count overflows).
    AllowLastOfLeeway(usize),
    /// The record should be allowed, the last `N` records were skipped as
    /// consecutive duplicates.
    AllowAfterSkipped(usize),
    /// The record should be skipped because no more consecutive duplicates
    /// are allowed.
    Skip,
}

impl Deduper {
    // Constructs a new [`Deduper`] that will skip duplicated entries after
    // some record has been received for the consecutive times specified by
    // `leeway`.
    pub fn with_leeway(leeway: NonZeroUsize) -> Self {
        Self {
            leeway,
            last_record: LastRecord {
                file: None,
                line: None,
                msg: String::new(),
            },
            duplicates: 0,
        }
    }

    /// Returns wether a record should be skipped or allowed.
    ///
    /// See [`DedupAction`].
    fn dedup(&mut self, record: &Record) -> DedupAction {
        let new_line = record.line();
        let new_file = record.file();
        let new_msg = record.args().to_string();
        if new_line == self.last_record.line
            && new_file == self.last_record.file.as_deref()
            && new_msg == self.last_record.msg
        {
            // Update dups count
            if let Some(updated_dups) = self.duplicates.checked_add(1) {
                self.duplicates = updated_dups;
            } else {
                let skipped = self.duplicates - self.leeway();
                self.duplicates = 0;
                return DedupAction::AllowAfterSkipped(skipped);
            }

            match self.duplicates.cmp(&self.leeway()) {
                Ordering::Less => DedupAction::Allow,
                Ordering::Equal => DedupAction::AllowLastOfLeeway(self.leeway()),
                Ordering::Greater => DedupAction::Skip,
            }
        } else {
            // Update last record
            self.last_record.file = new_file.map(ToOwned::to_owned);
            self.last_record.line = new_line;
            self.last_record.msg = new_msg;

            let dups = self.duplicates;
            self.duplicates = 0;

            match dups {
                n if n > self.leeway() => DedupAction::AllowAfterSkipped(n - self.leeway()),
                _ => DedupAction::Allow,
            }
        }
    }

    fn leeway(&self) -> usize {
        self.leeway.get()
    }
}

struct LastRecord {
    file: Option<String>,
    line: Option<u32>,
    msg: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_eq() {
        let leeway = NonZeroUsize::new(1).unwrap();
        let msg = format_args!("b");
        let mut deduper = Deduper::with_leeway(leeway);
        let record = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(msg)
            .build();
        let diff_file = Record::builder()
            .file(Some("b"))
            .line(Some(1))
            .args(msg)
            .build();
        let diff_line = Record::builder()
            .file(Some("b"))
            .line(Some(2))
            .args(msg)
            .build();
        let diff_msg = Record::builder()
            .file(Some("b"))
            .line(Some(2))
            .args(format_args!("diff msg"))
            .build();

        // First one is allowed
        assert_eq!(deduper.dedup(&record), DedupAction::Allow);
        // Second one is allowed because it comes from a diff file
        assert_eq!(deduper.dedup(&diff_file), DedupAction::Allow);
        // Third one is allowed because it comes from a diff line
        assert_eq!(deduper.dedup(&diff_line), DedupAction::Allow);
        // Fourth one is allowed because it has a diff msg
        assert_eq!(deduper.dedup(&diff_msg), DedupAction::Allow);
    }

    #[test]
    fn test_within_leeway_and_reset() {
        let leeway = NonZeroUsize::new(2).unwrap();
        let mut deduper = Deduper::with_leeway(leeway);
        let record_a = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("b"))
            .build();
        let record_b = Record::builder()
            .file(Some("b"))
            .line(Some(1))
            .args(format_args!("b"))
            .build();

        // All should be allowed as they are within leeway and dups are reset
        assert_eq!(deduper.dedup(&record_a), DedupAction::Allow);
        assert_eq!(deduper.dedup(&record_a), DedupAction::Allow);
        assert_eq!(deduper.dedup(&record_b), DedupAction::Allow);
        assert_eq!(deduper.dedup(&record_b), DedupAction::Allow);
        assert_eq!(deduper.dedup(&record_a), DedupAction::Allow);
        assert_eq!(deduper.dedup(&record_a), DedupAction::Allow);
    }

    #[test]
    fn test_leeway_warning() {
        let leeway = NonZeroUsize::new(4).unwrap();
        let mut deduper = Deduper::with_leeway(leeway);
        let dup = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("b"))
            .build();

        // First one should be allowed
        assert_eq!(deduper.dedup(&dup), DedupAction::Allow);
        // Silently allow the same log as long as leeway isn't met
        for _ in 0..(deduper.leeway() - 1) {
            assert_eq!(deduper.dedup(&dup), DedupAction::Allow);
        }
        // Allow last one within the leeway with a warning
        assert_eq!(
            deduper.dedup(&dup),
            DedupAction::AllowLastOfLeeway(deduper.leeway())
        );
    }

    #[test]
    fn test_dups() {
        let mut deduper = Deduper::with_leeway(NonZeroUsize::new(1).unwrap());
        let dup = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("b"))
            .build();
        let new_record = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("c"))
            .build();

        // First one should be allowed
        assert_eq!(deduper.dedup(&dup), DedupAction::Allow);
        // Second one should be the last one allowed because of the leeway
        assert_eq!(deduper.dedup(&dup), DedupAction::AllowLastOfLeeway(1));
        // Third one should be skipped
        assert_eq!(deduper.dedup(&dup), DedupAction::Skip);
        // A new log would be allowed with the summary of the skipped ones
        assert_eq!(
            deduper.dedup(&new_record),
            DedupAction::AllowAfterSkipped(1)
        );
    }

    #[test]
    fn test_overflowed_dups() {
        let mut deduper = Deduper::with_leeway(NonZeroUsize::new(1).unwrap());
        let dup = Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("b"))
            .build();

        // Bring dups to the edge of overflow
        deduper.duplicates = usize::MAX;

        // One more dup would overflow the usize, so next one is allowed
        assert_eq!(
            deduper.dedup(&dup),
            DedupAction::AllowAfterSkipped(usize::MAX - deduper.leeway())
        );

        // Dups are reset, next one is allowed as last under leeway
        assert_eq!(
            deduper.dedup(&dup),
            DedupAction::AllowLastOfLeeway(deduper.leeway())
        );
        assert_eq!(deduper.duplicates, 1);
    }
}
