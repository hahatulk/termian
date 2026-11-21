use std::{str, time::Duration};

use russh_sftp::{
    client::{error::Error, RawSftpSession},
    protocol::{FileAttributes, OpenFlags, StatusCode},
};

const PAGE_BYTES: usize = 32 * 1024;
const TEXT_ONLY: &str = "Preview supports UTF-8 text files. Use Open with... for other files.";
const FILE_CHANGED: &str = "The file changed. Close the preview and open it again.";

#[derive(Debug, PartialEq, Eq)]
pub struct TextPage {
    pub text: String,
    pub end: u64,
    pub size: Option<u64>,
    pub next: Option<u64>,
}

pub(super) async fn read(
    raw: &RawSftpSession,
    path: &str,
    offset: u64,
) -> Result<TextPage, String> {
    let read = |error: Error| error.to_string();
    let attrs = raw.lstat(path).await.map_err(read)?.attrs;
    if !attrs.file_type().is_file() {
        return Err("Select a regular file for preview.".into());
    }
    let handle = raw
        .open(path, OpenFlags::READ, FileAttributes::empty())
        .await
        .map_err(read)?
        .handle;
    let result =
        tokio::time::timeout(Duration::from_secs(15), read_page(raw, &handle, offset)).await;
    let _ = raw.close(handle).await;
    result.map_err(|_| "Reading the file preview timed out.".to_owned())?
}

async fn read_page(raw: &RawSftpSession, handle: &str, offset: u64) -> Result<TextPage, String> {
    let attrs = raw
        .fstat(handle)
        .await
        .map_err(|error| error.to_string())?
        .attrs;
    if !attrs.file_type().is_file() {
        return Err("Select a regular file for preview.".into());
    }
    let size = attrs.size;
    if offset > 0 && size.is_some_and(|size| offset >= size) {
        return Err(FILE_CHANGED.into());
    }
    let max = (PAGE_BYTES + 1) as u64;
    let count = size.map_or(max, |size| size.saturating_sub(offset).min(max));
    let mut bytes = Vec::with_capacity(count as usize);
    while bytes.len() < count as usize {
        let start = offset.checked_add(bytes.len() as u64).ok_or(FILE_CHANGED)?;
        let len = (count as usize - bytes.len()).min(PAGE_BYTES);
        match raw.read(handle, start, len as u32).await {
            Ok(data) if data.data.is_empty() => break,
            Ok(data) if data.data.len() <= len => bytes.extend(data.data),
            Ok(_) => return Err("The server sent more data than requested.".into()),
            Err(Error::Status(status)) if status.status_code == StatusCode::Eof => break,
            Err(error) => return Err(error.to_string()),
        }
    }
    if bytes.len() < count as usize && size.is_some() {
        return Err(FILE_CHANGED.into());
    }
    if offset != 0 && bytes.is_empty() {
        return Err(FILE_CHANGED.into());
    }
    decode_page(&bytes, offset, size)
}

fn decode_page(bytes: &[u8], offset: u64, size: Option<u64>) -> Result<TextPage, String> {
    let more = bytes.len() > PAGE_BYTES;
    let mut end = bytes.len().min(PAGE_BYTES);
    if more && bytes.get(end - 1) == Some(&b'\r') && bytes.get(end) == Some(&b'\n') {
        end -= 1;
    }
    let text = match str::from_utf8(&bytes[..end]) {
        Ok(text) => text,
        Err(error) if more && error.error_len().is_none() => {
            end = error.valid_up_to();
            str::from_utf8(&bytes[..end]).map_err(|_| TEXT_ONLY)?
        }
        Err(_) => return Err(TEXT_ONLY.into()),
    };
    if text
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\r' | '\n' | '\t'))
    {
        return Err(TEXT_ONLY.into());
    }
    let text = if offset == 0 {
        text.strip_prefix('\u{feff}').unwrap_or(text)
    } else {
        text
    };
    let end = offset.checked_add(end as u64).ok_or(FILE_CHANGED)?;
    Ok(TextPage {
        text: text.replace("\r\n", "\n"),
        end,
        size,
        next: more.then_some(end),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_split_utf8_character_starts_the_next_page() {
        let mut bytes = vec![b'a'; PAGE_BYTES - 1];
        bytes.extend("Привет".as_bytes());
        let size = bytes.len() as u64;
        let first = decode_page(&bytes[..PAGE_BYTES + 1], 0, Some(size)).unwrap();
        assert_eq!(first.text.len(), PAGE_BYTES - 1);
        assert_eq!(first.next, Some((PAGE_BYTES - 1) as u64));
        let offset = first.next.unwrap();
        let second = decode_page(&bytes[offset as usize..], offset, Some(size)).unwrap();
        assert_eq!(second.text, "Привет");
        assert_eq!(second.end, size);
        assert_eq!(second.next, None);
    }

    #[test]
    fn control_bytes_and_invalid_utf8_are_not_rendered() {
        assert!(decode_page(b"report\0", 0, None).is_err());
        assert!(decode_page(&[0xff], 0, None).is_err());
        assert!(decode_page(&[0xe2, 0x82], 0, None).is_err());
    }

    #[test]
    fn handles_empty_files_and_text_prefixes() {
        assert_eq!(
            decode_page(&[], 0, Some(0)).unwrap(),
            TextPage {
                text: String::new(),
                end: 0,
                size: Some(0),
                next: None
            }
        );
        let text = "\u{feff}one\r\ntwo\n";
        let page = decode_page(text.as_bytes(), 0, Some(text.len() as u64)).unwrap();
        assert_eq!(page.text, "one\ntwo\n");
        assert_eq!(page.end, text.len() as u64);
    }
}
