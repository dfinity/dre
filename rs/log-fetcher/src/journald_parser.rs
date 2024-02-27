use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Serialize, PartialEq)]
pub enum JournalField {
    Utf8(String),
    Binary(String),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct JournalEntry {
    pub fields: Vec<(String, JournalField)>,
}

pub fn parse_journal_entries_new(body: &[u8]) -> Vec<JournalEntry> {
    let mut entries = Vec::new();
    let mut current_entry = Vec::new();
    let mut current_line = Vec::new();

    let mut first_found = -2;

    let mut iter = body.iter();
    while let Some(byte) = iter.next() {
        match (byte, first_found) {
            (b'=', -1) => {
                current_line.push(*byte);
                first_found = 0;
            }
            (b'\n', -1) => {
                current_entry.push(current_line.clone());
                current_line.clear();
                let mut next = vec![];
                for _ in 0..8 {
                    let current = iter.next().unwrap();
                    next.push(*current);
                    current_line.push(*current)
                }

                let to_take =
                    i64::from_le_bytes([next[0], next[1], next[2], next[3], next[4], next[5], next[6], next[7]]);
                for _ in 0..to_take {
                    current_line.push(*iter.next().unwrap())
                }
                // To remove the added '\n' by format
                iter.next();
                current_entry.push(current_line.clone());
                current_line.clear();
                first_found = -2;
            }
            (b'\n', 0) => {
                current_entry.push(current_line.clone());
                current_line.clear();
                first_found = -2;
            }
            (b'\n', -2) => {
                if let Some(entry) = parse_journal_entry(current_entry.as_slice()) {
                    entries.push(entry);
                }
                current_entry.clear();
                current_line.clear();
                first_found = -2;
            }
            (_, -1) | (_, 0) => current_line.push(*byte),
            (_, -2) => {
                current_line.push(*byte);
                first_found = -1;
            }
            (a, b) => unreachable!("Shouldn't happen: {}, {}", a, b),
        }
    }
    // Check if there's an entry at the end of the body
    if !current_entry.is_empty() {
        if let Some(entry) = parse_journal_entry(&current_entry) {
            entries.push(entry);
        }
    }

    entries
}

pub fn _parse_journal_entries(body: &[u8]) -> Vec<JournalEntry> {
    let mut entries = Vec::new();
    let mut current_entry = Vec::new();
    let lines: Vec<_> = body.split(|&c| c == b'\n').collect();
    let mut lines = VecDeque::from(lines);

    while let Some(line) = lines.pop_front() {
        if line.is_empty() {
            // Empty line indicates the end of an entry
            if !current_entry.is_empty() {
                if let Some(entry) = parse_journal_entry(&current_entry) {
                    entries.push(entry);
                }
                current_entry.clear();
            }
        } else {
            // Non-empty line, add it to the current entry
            current_entry.push(line.to_vec());
        }
    }

    // Check if there's an entry at the end of the body
    if !current_entry.is_empty() {
        if let Some(entry) = parse_journal_entry(&current_entry) {
            entries.push(entry);
        }
    }

    entries
}

fn parse_journal_entry(entry_lines: &[Vec<u8>]) -> Option<JournalEntry> {
    let mut entry = JournalEntry { fields: Vec::new() };

    println!(
        "Received lines: \n{:?}",
        entry_lines
            .into_iter()
            .map(|line| String::from_utf8_lossy(line))
            .collect::<Vec<_>>()
    );

    let mut iter = entry_lines.iter();
    while let Some(line) = iter.next() {
        if line.contains(&b'=') {
            // Normal field
            if let Some((field_name, field_data)) = parse_normal(line) {
                entry.fields.push((field_name, field_data));
            }
            continue;
        }

        // Binary field
        // The body is always split into multiple lines, in the calling function.
        // Consequently, the binary field may span several lines, and needs to be reassembled.
        // According to the systemd journal export format (https://systemd.io/JOURNAL_EXPORT_FORMATS/),
        // the initial line contains only the field name followed by '\n'.
        // Subsequently, a 64-bit little-endian number in the next line indicates the field's size.
        // The field's actual content immediately follows this size descriptor.
        // During data processing, it's necessary to reintroduce '\n' after each
        // iteration because it's removed by the .split('\n') call in the calling function.
        let mut multiline = vec![];
        let name = String::from_utf8_lossy(line).to_string();
        let next = iter.next().unwrap();
        let mut size = i64::from_le_bytes([next[0], next[1], next[2], next[3], next[4], next[5], next[6], next[7]]);
        let remaining = &next[8..];
        multiline.extend(remaining);
        size = size.wrapping_sub(remaining.len() as i64);

        while size > 0 {
            if let Some(val) = iter.next() {
                multiline.push(b'\n');
                multiline.extend(val);
                size = size.wrapping_sub(val.len() as i64).wrapping_sub(1);
            } else {
                break;
            }
        }

        entry.fields.push((
            name,
            JournalField::Binary(String::from_utf8_lossy(&multiline).to_string()),
        ))
    }

    if entry.fields.is_empty() {
        None
    } else {
        Some(entry)
    }
}

fn parse_normal(data: &[u8]) -> Option<(String, JournalField)> {
    let mut iter = data.splitn(2, |&c| c == b'=');

    if let Some(field_name_bytes) = iter.next() {
        let field_name = String::from_utf8_lossy(field_name_bytes).trim().to_string();

        if let Some(field_data_bytes) = iter.next() {
            let field_data = JournalField::Utf8(String::from_utf8_lossy(field_data_bytes).to_string());

            return Some((field_name.trim().to_string(), field_data));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;

    pub fn serialize_string_field<W: Write>(
        field_name: &str,
        field_data: &str,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        // Serialize as binary field
        writer.write_all(field_name.as_bytes())?;
        writer.write_all(b"\n")?;

        let size = field_data.len() as u64;
        let size_bytes: [u8; 8] = size.to_le_bytes();
        writer.write_all(&size_bytes)?;
        writer.write_all(field_data.as_bytes())?;
        writer.write_all(b"\n")?;

        Ok(())
    }

    #[test]
    fn test_parse_journal_entries() {
        // Test case with two entries
        let body = b"field1=value1\nfield2=value2\n\nfield3=value3\nfield4=\n";
        let entries = parse_journal_entries_new(body);
        assert_eq!(entries.len(), 2);

        // Verify the first entry
        let entry1 = &entries[0];
        assert_eq!(entry1.fields.len(), 2);
        assert_eq!(
            entry1.fields[0],
            ("field1".to_string(), JournalField::Utf8("value1".to_string()))
        );
        assert_eq!(
            entry1.fields[1],
            ("field2".to_string(), JournalField::Utf8("value2".to_string()))
        );

        // Verify the second entry
        let entry2 = &entries[1];
        assert_eq!(entry2.fields.len(), 2);
        assert_eq!(
            entry2.fields[0],
            ("field3".to_string(), JournalField::Utf8("value3".to_string()))
        );
        assert_eq!(
            entry2.fields[1],
            ("field4".to_string(), JournalField::Utf8("".to_string()))
        );
    }

    #[test]
    fn test_parse_journal_entries_binary() {
        // Test case with binary data

        let mut body = vec![];
        let mut serialized_data = Vec::new();
        body.extend(b"field1=value1\n");
        serialize_string_field("MESSAGE", "foo\nbar", &mut serialized_data).unwrap();
        body.extend(serialized_data.clone());

        let entries = parse_journal_entries_new(&body);
        assert_eq!(entries.len(), 1);

        // Verify the entry with binary data
        let entry = &entries[0];
        assert_eq!(entry.fields.len(), 2);
        assert_eq!(
            entry.fields[0],
            ("field1".to_string(), JournalField::Utf8("value1".to_string()))
        );
        assert_eq!(
            entry.fields[1],
            ("MESSAGE".to_string(), JournalField::Binary("foo\nbar".to_string()))
        );
    }

    #[test]
    fn test_parse_journal_entries_binary_larger() {
        // Test case with binary data

        let mut body = vec![];
        let mut serialized_data = Vec::new();
        body.extend(b"__CURSOR=s=bcce4fb8ffcb40e9a6e05eee8b7831bf;i=5ef603;b=ec25d6795f0645619ddac9afdef453ee;m=545242e7049;t=50f1202\n");
        body.extend(b"__REALTIME_TIMESTAMP=1423944916375353\n");
        body.extend(b"_SYSTEMD_OWNER_UID=1001\n");
        serialize_string_field("OTHER_BIN", "some random data\nbar", &mut serialized_data).unwrap();
        body.extend(serialized_data.clone());
        body.extend(b"_AUDIT_LOGINUID=1001\n");
        body.extend(b"SYSLOG_IDENTIFIER=python3\n");
        serialized_data.clear();
        serialize_string_field("MESSAGE", "foo\nbar", &mut serialized_data).unwrap();
        body.extend(serialized_data);

        let entries = parse_journal_entries_new(&body);
        assert_eq!(entries.len(), 1);

        // Verify the entry with binary data
        let entry = &entries[0];
        assert_eq!(entry.fields.len(), 7);
        assert_eq!(
            entry.fields[0],
            ("__CURSOR".to_string(), JournalField::Utf8("s=bcce4fb8ffcb40e9a6e05eee8b7831bf;i=5ef603;b=ec25d6795f0645619ddac9afdef453ee;m=545242e7049;t=50f1202".to_string()))
        );
        assert_eq!(
            entry.fields[1],
            (
                "__REALTIME_TIMESTAMP".to_string(),
                JournalField::Utf8("1423944916375353".to_string())
            )
        );
        assert_eq!(
            entry.fields[2],
            ("_SYSTEMD_OWNER_UID".to_string(), JournalField::Utf8("1001".to_string()))
        );
        assert_eq!(
            entry.fields[3],
            (
                "OTHER_BIN".to_string(),
                JournalField::Binary("some random data\nbar".to_string())
            )
        );
        assert_eq!(
            entry.fields[4],
            ("_AUDIT_LOGINUID".to_string(), JournalField::Utf8("1001".to_string()))
        );
        assert_eq!(
            entry.fields[5],
            (
                "SYSLOG_IDENTIFIER".to_string(),
                JournalField::Utf8("python3".to_string())
            )
        );
        assert_eq!(
            entry.fields[6],
            ("MESSAGE".to_string(), JournalField::Binary("foo\nbar".to_string()))
        );
    }

    #[test]
    fn test_parse_journal_entries_binary_field_with_newline_end() {
        // Test case with binary data

        let mut body = vec![];
        let mut serialized_data = Vec::new();
        body.extend(b"__CURSOR=s=bcce4fb8ffcb40e9a6e05eee8b7831bf;i=5ef603;b=ec25d6795f0645619ddac9afdef453ee;m=545242e7049;t=50f1202\n");
        body.extend(b"__REALTIME_TIMESTAMP=1423944916375353\n");
        body.extend(b"_SYSTEMD_OWNER_UID=1001\n");
        serialize_string_field("OTHER_BIN", "some random data\nbar\n", &mut serialized_data).unwrap();
        body.extend(serialized_data.clone());
        body.extend(b"_AUDIT_LOGINUID=1001\n");
        body.extend(b"SYSLOG_IDENTIFIER=python3\n");
        serialized_data.clear();
        serialize_string_field("MESSAGE", "foo\nbar", &mut serialized_data).unwrap();
        body.extend(serialized_data);

        let entries = parse_journal_entries_new(&body);
        assert_eq!(entries.len(), 1);

        // Verify the entry with binary data
        let entry = &entries[0];
        assert_eq!(entry.fields.len(), 7);
        assert_eq!(
            entry.fields[0],
            ("__CURSOR".to_string(), JournalField::Utf8("s=bcce4fb8ffcb40e9a6e05eee8b7831bf;i=5ef603;b=ec25d6795f0645619ddac9afdef453ee;m=545242e7049;t=50f1202".to_string()))
        );
        assert_eq!(
            entry.fields[1],
            (
                "__REALTIME_TIMESTAMP".to_string(),
                JournalField::Utf8("1423944916375353".to_string())
            )
        );
        assert_eq!(
            entry.fields[2],
            ("_SYSTEMD_OWNER_UID".to_string(), JournalField::Utf8("1001".to_string()))
        );
        assert_eq!(
            entry.fields[3],
            (
                "OTHER_BIN".to_string(),
                JournalField::Binary("some random data\nbar\n".to_string())
            )
        );
        assert_eq!(
            entry.fields[4],
            ("_AUDIT_LOGINUID".to_string(), JournalField::Utf8("1001".to_string()))
        );
        assert_eq!(
            entry.fields[5],
            (
                "SYSLOG_IDENTIFIER".to_string(),
                JournalField::Utf8("python3".to_string())
            )
        );
        assert_eq!(
            entry.fields[6],
            ("MESSAGE".to_string(), JournalField::Binary("foo\nbar".to_string()))
        );
    }

    #[test]
    fn test_parse_journal_entries_empty() {
        // Test case with empty body
        let body = b"";
        let entries = parse_journal_entries_new(body);
        assert_eq!(entries.len(), 0);
    }
}
