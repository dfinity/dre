use serde::Serialize;
use std::collections::VecDeque;

#[derive(Debug, Serialize)]
pub enum JournalField {
    Utf8(String),
    Binary(String),
}

#[derive(Debug, Serialize)]
pub struct JournalEntry {
    pub fields: Vec<(String, JournalField)>,
}

pub fn parse_journal_entries(body: &[u8]) -> Vec<JournalEntry> {
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

    for line in entry_lines {
        if let Some((field_name, field_data)) = parse_journal_field(line) {
            entry.fields.push((field_name, field_data));
        }
    }

    if entry.fields.is_empty() {
        None
    } else {
        Some(entry)
    }
}

fn parse_journal_field(data: &[u8]) -> Option<(String, JournalField)> {
    let mut iter = data.splitn(2, |&c| c == b'=');

    if let Some(field_name_bytes) = iter.next() {
        let field_name = String::from_utf8_lossy(field_name_bytes).trim().to_string();

        if let Some(field_data_bytes) = iter.next() {
            let field_data = if field_data_bytes.starts_with(b"\n") {
                let (size_bytes, rest) = field_data_bytes[1..].split_at(8);
                let size = u64::from_le_bytes([
                    size_bytes[0],
                    size_bytes[1],
                    size_bytes[2],
                    size_bytes[3],
                    size_bytes[4],
                    size_bytes[5],
                    size_bytes[6],
                    size_bytes[7],
                ]);

                if let Some(binary_data) = rest.get(1..(size as usize + 1)) {
                    JournalField::Binary(String::from_utf8_lossy(binary_data).to_string())
                } else {
                    return None;
                }
            } else {
                JournalField::Utf8(String::from_utf8_lossy(field_data_bytes).to_string())
            };

            return Some((field_name.trim().to_string(), field_data));
        }
    }

    None
}
