use colored::Colorize;
use fs_err;
use log::info;
use similar::{ChangeTag, TextDiff};

pub struct Writer {
    writer: Box<dyn std::io::Write>,
    use_color: bool,
}

impl Writer {
    pub fn new(output: &Option<std::path::PathBuf>, use_color: bool) -> anyhow::Result<Writer> {
        match output {
            Some(path) => {
                let file = fs_err::File::create(path)?;
                info!("Writing to file: {:?}", path.canonicalize()?);

                return Ok(Writer {
                    writer: Box::new(std::io::BufWriter::new(file)),
                    use_color,
                });
            }
            None => Ok(Writer {
                writer: Box::new(std::io::stdout()),
                use_color,
            }),
        }
    }

    pub fn write_line(&mut self, line: &str) -> anyhow::Result<()> {
        write!(self.writer, "{}", line)?;
        Ok(())
    }

    pub fn write_diff(&mut self, diff: &TextDiff<'_, '_, '_, str>) -> anyhow::Result<()> {
        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                self.write_line(&"---".dimmed())?;
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let line = &change.to_string();
                    let (sign, color) = match change.tag() {
                        ChangeTag::Delete => ("-", colored::Color::Red),
                        ChangeTag::Insert => ("+", colored::Color::Green),
                        ChangeTag::Equal => (" ", colored::Color::White),
                    };
                    if self.use_color {
                        write!(self.writer, "{}{}", sign.color(color), line.color(color))?;
                    } else {
                        write!(self.writer, "{}{}", sign, line)?;
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)] // Used in tests
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use similar::TextDiff;
    use std::path::PathBuf;

    #[test]
    fn test_generate_diff() {
        // Test data
        let json1 = serde_json::json!({
            "a": 1,
            "b": 2
        });
        let json2 = serde_json::json!({
            "a": 1,
            "b": 3
        });

        // Generate diff
        let json1_str: String = serde_json::to_string_pretty(&json1).unwrap();
        let json2_str: String = serde_json::to_string_pretty(&json2).unwrap();
        let diff = TextDiff::from_lines(json1_str.as_str(), json2_str.as_str());

        // Write diff to file
        let mut writer = Writer::new(&Some(PathBuf::from("/tmp/diff_test_output.json")), false).unwrap();
        writer.write_diff(&diff).unwrap();
        // Flush data to disk
        writer.flush().unwrap(); // Ensure data is written to disk
        drop(writer); // Explicitly drop to ensure file is closed

        // Read diff output from file
        let diff_output = fs_err::read_to_string("/tmp/diff_test_output.json").unwrap();

        // Assert diff output contains expected changes
        assert!(diff_output.contains("  \"a\": 1,"));
        assert!(diff_output.contains("-  \"b\": 2"));
        assert!(diff_output.contains("+  \"b\": 3"));

        // Cleanup
        fs_err::remove_file("/tmp/diff_test_output.json").unwrap();
    }
}
