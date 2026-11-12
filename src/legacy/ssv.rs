//! Decoder for WebKit's SerializedScriptValue, the format WebKit uses for
//! IndexedDB record values. Only the plain-data subset the legacy profile
//! stores is supported: objects, arrays, strings, numbers, booleans, null.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

const ARRAY: u8 = 1;
const OBJECT: u8 = 2;
const UNDEFINED: u8 = 3;
const NULL: u8 = 4;
const INT: u8 = 5;
const ZERO: u8 = 6;
const ONE: u8 = 7;
const FALSE: u8 = 8;
const TRUE: u8 = 9;
const DOUBLE: u8 = 10;
const DATE: u8 = 11;
const STRING: u8 = 16;
const EMPTY_STRING: u8 = 17;
const OBJECT_REFERENCE: u8 = 19;

const TERMINATOR: u32 = 0xFFFF_FFFF;
const STRING_POOL: u32 = 0xFFFF_FFFE;
const NON_INDEX_PROPERTIES: u32 = 0xFFFF_FFFD;
const LATIN1_FLAG: u32 = 0x8000_0000;

pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    let mut reader = Reader { bytes, pos: 0, strings: Vec::new(), objects: Vec::new() };
    reader.u32()?; // format version
    reader.value()
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    strings: Vec<String>,
    objects: Vec<Value>,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.pos.checked_add(n).filter(|&end| end <= self.bytes.len()).ok_or("unexpected end of value")?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// Pool indices are as wide as the pool size requires.
    fn pool_index(&mut self, len: usize) -> Result<usize, String> {
        Ok(if len <= 0xFF {
            self.u8()? as usize
        } else if len <= 0xFFFF {
            u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize
        } else {
            self.u32()? as usize
        })
    }

    /// Reads string data whose length word was already consumed.
    fn string_data(&mut self, len: u32) -> Result<String, String> {
        if len == STRING_POOL {
            let index = self.pool_index(self.strings.len())?;
            return self.strings.get(index).cloned().ok_or_else(|| format!("bad string pool index {index}"));
        }
        let count = (len & !LATIN1_FLAG) as usize;
        let s = if len & LATIN1_FLAG != 0 {
            self.take(count)?.iter().map(|&b| b as char).collect()
        } else {
            let units: Vec<u16> =
                self.take(count * 2)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        };
        self.strings.push(s.clone());
        Ok(s)
    }

    fn properties(&mut self, map: &mut BTreeMap<String, Value>) -> Result<(), String> {
        loop {
            let len = self.u32()?;
            if len == TERMINATOR {
                return Ok(());
            }
            let key = self.string_data(len)?;
            let value = self.value()?;
            map.insert(key, value);
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        Ok(match self.u8()? {
            UNDEFINED => Value::Undefined,
            NULL => Value::Null,
            INT => Value::Number(i32::from_le_bytes(self.take(4)?.try_into().unwrap()) as f64),
            ZERO => Value::Number(0.),
            ONE => Value::Number(1.),
            FALSE => Value::Bool(false),
            TRUE => Value::Bool(true),
            DOUBLE | DATE => Value::Number(self.f64()?),
            STRING => {
                let len = self.u32()?;
                Value::String(self.string_data(len)?)
            }
            EMPTY_STRING => Value::String(String::new()),
            OBJECT => {
                let slot = self.objects.len();
                self.objects.push(Value::Null);
                let mut map = BTreeMap::new();
                self.properties(&mut map)?;
                let value = Value::Object(map);
                self.objects[slot] = value.clone();
                value
            }
            ARRAY => {
                let slot = self.objects.len();
                self.objects.push(Value::Null);
                let len = self.u32()? as usize;
                let mut items = vec![Value::Undefined; len];
                loop {
                    let index = self.u32()?;
                    if index == TERMINATOR {
                        break;
                    }
                    if index == NON_INDEX_PROPERTIES {
                        self.properties(&mut BTreeMap::new())?;
                        break;
                    }
                    let item = self.value()?;
                    if let Some(entry) = items.get_mut(index as usize) {
                        *entry = item;
                    }
                }
                let value = Value::Array(items);
                self.objects[slot] = value.clone();
                value
            }
            OBJECT_REFERENCE => {
                let index = self.pool_index(self.objects.len())?;
                self.objects.get(index).cloned().ok_or_else(|| format!("bad object reference {index}"))?
            }
            tag => return Err(format!("unsupported value tag {tag}")),
        })
    }
}

/// Decodes an IndexedDB record key; the legacy profile keys records by number.
pub fn decode_number_key(bytes: &[u8]) -> Option<f64> {
    match bytes {
        [0x00, 0x20, rest @ ..] if rest.len() == 8 => Some(f64::from_le_bytes(rest.try_into().ok()?)),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Minimal encoder for building fixtures; mirrors what WebKit writes.
    #[derive(Default)]
    pub struct Encoder {
        pub out: Vec<u8>,
        pool: Vec<String>,
    }

    impl Encoder {
        pub fn new() -> Self {
            let mut e = Self::default();
            e.out.extend(15u32.to_le_bytes());
            e
        }
        pub fn str_data(&mut self, s: &str) {
            if let Some(i) = self.pool.iter().position(|p| p == s) {
                self.out.extend(STRING_POOL.to_le_bytes());
                self.out.push(i as u8);
            } else {
                self.out.extend((s.len() as u32 | LATIN1_FLAG).to_le_bytes());
                self.out.extend(s.bytes());
                self.pool.push(s.into());
            }
        }
        pub fn begin_object(&mut self) {
            self.out.push(OBJECT);
        }
        pub fn key(&mut self, k: &str) {
            self.str_data(k);
        }
        pub fn string(&mut self, s: &str) {
            self.out.push(STRING);
            self.str_data(s);
        }
        pub fn int(&mut self, n: i32) {
            self.out.push(INT);
            self.out.extend(n.to_le_bytes());
        }
        pub fn tag(&mut self, t: u8) {
            self.out.push(t);
        }
        pub fn end(&mut self) {
            self.out.extend(TERMINATOR.to_le_bytes());
        }
    }

    #[test]
    fn decodes_host_like_record_with_pooled_keys() {
        let mut e = Encoder::new();
        e.begin_object();
        e.key("label");
        e.string("BAEcipher");
        e.key("group");
        e.begin_object();
        e.key("local_id");
        e.int(39);
        e.key("id");
        e.tag(UNDEFINED);
        e.end();
        e.key("ssh_config");
        e.begin_object();
        e.key("local_id"); // pooled reference
        e.int(220);
        e.end();
        e.key("is_shared");
        e.tag(FALSE);
        e.end();

        let value = decode(&e.out).unwrap();
        assert_eq!(value.get("label").and_then(Value::as_str), Some("BAEcipher"));
        assert_eq!(value.get("group").and_then(|g| g.get("local_id")).and_then(Value::as_f64), Some(39.));
        assert_eq!(value.get("ssh_config").and_then(|g| g.get("local_id")).and_then(Value::as_f64), Some(220.));
        assert_eq!(value.get("is_shared"), Some(&Value::Bool(false)));
    }

    #[test]
    fn decodes_utf16_strings_and_arrays() {
        let mut out = 15u32.to_le_bytes().to_vec();
        out.push(ARRAY);
        out.extend(2u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.push(STRING);
        let word: Vec<u16> = "хост".encode_utf16().collect();
        out.extend((word.len() as u32).to_le_bytes());
        word.iter().for_each(|u| out.extend(u.to_le_bytes()));
        out.extend(1u32.to_le_bytes());
        out.push(NULL);
        out.extend(TERMINATOR.to_le_bytes());
        assert_eq!(decode(&out).unwrap(), Value::Array(vec![Value::String("хост".into()), Value::Null]));
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let mut e = Encoder::new();
        e.begin_object();
        e.key("label");
        assert!(decode(&e.out).is_err());
    }

    #[test]
    fn decodes_numeric_record_key() {
        let mut key = vec![0x00, 0x20];
        key.extend(220f64.to_le_bytes());
        assert_eq!(decode_number_key(&key), Some(220.));
        assert_eq!(decode_number_key(&[0x00, 0x10]), None);
    }
}
