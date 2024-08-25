/// Syslog Facility, according to [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
///
/// Note that the original integer values are already multiplied by 8.
#[derive(Copy, Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum SyslogFacility {
    /// kernel messages.
    Kernel = 0 << 3,
    /// user-level messages.
    UserLevel = 1 << 3,
    /// mail system.
    MailSystem = 2 << 3,
    /// system daemons.
    SystemDaemons = 3 << 3,
    /// security/authorization messages.
    Authorization = 4 << 3,
    /// messages generated internally by syslogd.
    SyslogD = 5 << 3,
    /// line printer subsystem.
    LinePrinter = 6 << 3,
    /// network news subsystem.
    News = 7 << 3,
    /// UUCP subsystem.
    Uucp = 8 << 3,
    /// clock daemon.
    Clock = 9 << 3,
    /// security/authorization messages.
    Authorization2 = 10 << 3,
    /// FTP daemon.
    Ftp = 11 << 3,
    /// NTP subsystem.
    Ntp = 12 << 3,
    /// log audit.
    LogAudit = 13 << 3,
    /// log alert.
    LogAlert = 14 << 3,
    /// clock daemon (note 2).
    Clock2 = 15 << 3,
    /// local use 0  (local0).
    LocalUse0 = 16 << 3,
    /// local use 1  (local1).
    LocalUse1 = 17 << 3,
    /// local use 2  (local2).
    LocalUse2 = 18 << 3,
    /// local use 3  (local3).
    LocalUse3 = 19 << 3,
    /// local use 4  (local4).
    LocalUse4 = 20 << 3,
    /// local use 5  (local5).
    LocalUse5 = 21 << 3,
    /// local use 6  (local6).
    LocalUse6 = 22 << 3,
    /// local use 7  (local7).
    LocalUse7 = 23 << 3,
}
