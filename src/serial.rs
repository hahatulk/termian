//! Serial connections as in legacy: the «Serial» form's options
//! (`defaultSerialOptions`), its baud rates and choices, the charsets this
//! client can translate, and the checks the legacy serial worker makes
//! before it opens a port.

use serde::{Deserialize, Serialize};

/// The legacy «Baud rate» suggestions.
pub const BAUD_RATES: [u32; 17] = [50, 75, 110, 134, 150, 200, 300, 600, 1200, 1800, 2400, 4800, 9600, 19200, 38400, 57600, 115200];
pub const DATA_BITS: [u8; 4] = [8, 7, 6, 5];
pub const STOP_BITS: [u8; 2] = [1, 2];

/// Legacy `supportedCharsets`.
const LEGACY_CHARSETS: [&str; 73] = [
    "UTF-8", "Big5", "Big5-HKSCS", "CP437", "CP737", "CP775", "CP808", "CP850", "CP852", "CP855", "CP856", "CP857", "CP858", "CP860", "CP861", "CP862", "CP863", "CP864", "CP865", "CP866",
    "CP869", "CP922", "CP936", "CP949", "CP950", "CP1046", "CP1124", "CP1125", "CP1129", "CP1133", "CP1161", "CP1162", "CP1163", "EUC-CN", "EUC-JP", "EUC-KR", "GB18030", "GBK", "ISO-646CN",
    "ISO-646JP", "ISO-8859-1", "ISO-8859-2", "ISO-8859-3", "ISO-8859-4", "ISO-8859-5", "ISO-8859-6", "ISO-8859-7", "ISO-8859-8", "ISO-8859-9", "ISO-8859-10", "ISO-8859-11", "ISO-8859-13",
    "ISO-8859-14", "ISO-8859-15", "ISO-8859-16", "KOI8-R", "KOI8-RU", "KOI8-T", "KOI8-U", "PT154", "RK1048", "Shift_JIS", "TIS-620", "windows-1250", "windows-1251", "windows-1252",
    "windows-1253", "windows-1254", "windows-1255", "windows-1256", "windows-1257", "windows-1258", "windows-874",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

impl Parity {
    pub const ALL: [Parity; 3] = [Parity::None, Parity::Odd, Parity::Even];

    /// The legacy menu text.
    pub fn label(self) -> &'static str {
        match self {
            Parity::None => "No",
            Parity::Odd => "Odd",
            Parity::Even => "Even",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    #[default]
    None,
    RtsCts,
    /// Offered as in legacy, which opens such a port without flow control.
    DsrDtr,
    XonXoff,
}

impl FlowControl {
    pub const ALL: [FlowControl; 4] = [FlowControl::None, FlowControl::RtsCts, FlowControl::DsrDtr, FlowControl::XonXoff];

    pub fn label(self) -> &'static str {
        match self {
            FlowControl::None => "None",
            FlowControl::RtsCts => "RTS/CTS",
            FlowControl::DsrDtr => "DSR/DTR",
            FlowControl::XonXoff => "XON/XOFF",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerialOptions {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: Parity,
    pub flow_control: FlowControl,
    pub charset: String,
}

impl Default for SerialOptions {
    fn default() -> Self {
        Self { path: String::new(), baud_rate: 9600, data_bits: 8, stop_bits: 1, parity: Parity::None, flow_control: FlowControl::None, charset: "UTF-8".into() }
    }
}

impl SerialOptions {
    /// «Connect» is enabled with a port and a baud rate, as in legacy.
    pub fn can_connect(&self) -> bool {
        !self.path.is_empty() && self.baud_rate > 0
    }

    /// The legacy serial worker's checks.
    pub fn validate(&self) -> Result<(), String> {
        if self.path.is_empty()
            || self.path.len() > 260
            || self.path.chars().any(char::is_control)
            || !(75..=4_000_000).contains(&self.baud_rate)
            || !(5..=8).contains(&self.data_bits)
            || !STOP_BITS.contains(&self.stop_bits)
        {
            return Err("Invalid serial settings: check port, baud, data bits, parity, stop bits and flow control".into());
        }
        if encoding(&self.charset).is_none() {
            return Err(format!("Charset \"{}\" is not supported", self.charset));
        }
        Ok(())
    }

    /// Text for the tab and the Logs row: the port's last path part.
    pub fn label(&self) -> String {
        self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path).to_owned()
    }
}

/// The legacy charsets this client translates (encoding_rs knows their names).
pub fn charsets() -> Vec<&'static str> {
    LEGACY_CHARSETS.iter().copied().filter(|name| encoding(name).is_some()).collect()
}

/// Legacy's charset field takes the first known name containing the typed text, else UTF-8.
pub fn charset_for(typed: &str) -> &'static str {
    let typed = typed.to_lowercase();
    charsets().into_iter().find(|name| name.to_lowercase().contains(&typed)).unwrap_or("UTF-8")
}

pub fn encoding(charset: &str) -> Option<&'static encoding_rs::Encoding> {
    let label = match charset {
        "CP866" => "ibm866",
        "CP936" => "gbk",
        "CP949" => "euc-kr",
        "CP950" => "big5",
        "EUC-CN" => "gb2312",
        "TIS-620" => "windows-874",
        other => other,
    };
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// Serial ports the system lists, sorted; only these can be opened.
pub fn list_ports() -> Vec<String> {
    let mut ports: Vec<String> = serial2::SerialPort::available_ports().unwrap_or_default().into_iter().map(|p| p.to_string_lossy().into_owned()).collect();
    ports.sort();
    ports.dedup();
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_checks_follow_legacy() {
        let options = SerialOptions::default();
        assert_eq!((options.baud_rate, options.data_bits, options.stop_bits, options.charset.as_str()), (9600, 8, 1, "UTF-8"));
        assert!(!options.can_connect(), "a port is required");
        let port = SerialOptions { path: "/dev/tty.usbserial-1".into(), ..SerialOptions::default() };
        assert!(port.can_connect() && port.validate().is_ok());
        assert_eq!(port.label(), "tty.usbserial-1");
        assert!(SerialOptions { baud_rate: 50, ..port.clone() }.validate().is_err(), "legacy refuses below 75");
        assert!(SerialOptions { stop_bits: 3, ..port.clone() }.validate().is_err());
        assert!(SerialOptions { path: "a\u{7}".into(), ..port.clone() }.validate().is_err());
        assert!(SerialOptions { charset: "CP437".into(), ..port }.validate().unwrap_err().contains("not supported"));
    }

    #[test]
    fn charsets_are_the_translatable_legacy_ones() {
        let list = charsets();
        assert_eq!(list[0], "UTF-8");
        for name in ["windows-1251", "KOI8-R", "CP866", "Shift_JIS", "ISO-8859-5", "GB18030"] {
            assert!(list.contains(&name), "{name}");
        }
        assert!(!list.contains(&"CP437"));
        assert_eq!(charset_for("1251"), "windows-1251");
        assert_eq!(charset_for("nothing"), "UTF-8");
    }
}
