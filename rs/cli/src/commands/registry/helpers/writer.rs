use colored::Colorize;
use fs_err;
use log::info;
use similar::ChangeTag;

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

    pub fn write_diff_line(&mut self, change_tag: &ChangeTag, line: &str) -> anyhow::Result<()> {
        let (sign, color) = match change_tag {
            ChangeTag::Delete => ("-", colored::Color::Red),
            ChangeTag::Insert => ("+", colored::Color::Green),
            ChangeTag::Equal => (" ", colored::Color::White),
        };
        if self.use_color {
            write!(self.writer, "{}{}", sign.color(color), line.color(color))?;
        } else {
            write!(self.writer, "{}{}", sign, line)?;
        }
        Ok(())
    }

    #[allow(dead_code)] // Used in tests
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
