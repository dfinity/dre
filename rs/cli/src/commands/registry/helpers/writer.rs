use log::info;
use fs_err;

pub(crate) fn create_writer(output: &Option<std::path::PathBuf>) -> anyhow::Result<Box<dyn std::io::Write>> {
    match output {
        Some(path) => {
            let file = fs_err::File::create(path)?;
            info!("Writing to file: {:?}", file.path().canonicalize()?);
            Ok(Box::new(std::io::BufWriter::new(file)))
        }
        None => Ok(Box::new(std::io::stdout())),
    }
}