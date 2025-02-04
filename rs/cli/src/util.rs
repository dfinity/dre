use std::{
    io::Error,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use colored::Colorize;
use dialoguer::Confirm;
use futures::future::{join_all, BoxFuture};
use futures::FutureExt;
use log::debug;
use tokio::io::AsyncWriteExt as TokioAsyncWriteExt;
use tokio::{
    io::AsyncReadExt as _,
    process::Command,
    sync::Mutex,
    task::{spawn_blocking, JoinHandle},
};

/// Convert a vector of bytes to UTF-8, casting the error to an anyhow::Error.
pub fn utf8(res: Vec<u8>, errmsg: &str) -> anyhow::Result<String> {
    String::from_utf8(res).map_err(|e| anyhow::anyhow!("{}: {}", errmsg, e))
}

/// Given a writeup in Markdown, it extracts (if any) the first level-1 heading (# Title)
/// and returns the title and the remainder of the text.  If no title is found at the
/// beginning of the text, the original text is returned near-unchanged.
pub fn extract_title_and_text(markdown_text: &str) -> (Option<String>, String) {
    // Step 1: Remove leading/trailing empty lines (including lines with only whitespace).
    let lines: Vec<&str> = markdown_text.lines().skip_while(|line| line.trim().is_empty()).collect();
    let lines: Vec<&str> = lines.into_iter().rev().skip_while(|line| line.trim().is_empty()).collect();
    let lines: Vec<&str> = lines.into_iter().rev().collect();

    // Step 2: Parse the first line as a title if it starts with '# '
    if lines.is_empty() {
        // If no lines remain after trimming, there's no title or body
        return (None, "".to_string());
    }

    let first_line = lines[0];
    // CommonMark-compliant H1 headings MUST start with "# ", so starts_with("# ") would be enough.
    // To tolerate other “flavors” of Markdown that allow #Title without a space, we use a more complex check.
    let (title, body) = if first_line.starts_with('#') && !first_line.starts_with("##") {
        // Strip out '#' plus any extra leading space
        let stripped_title = first_line.trim_start_matches('#').trim_start();
        let body = lines[1..].join("\n"); // Join the remaining lines
        (Some(stripped_title.to_owned()), body.trim_start().to_string())
    } else {
        (None, lines.join("\n"))
    };
    (title, body)
}

/// Ask the user a question interactively, returning t/f to the y/n question.
/// This function spawns a thread to avoid blocking the program.
/// Avoid calling this during tests or when the program runs under no tty.
pub fn yesno(question: &str, default: bool) -> JoinHandle<anyhow::Result<bool>> {
    let q = question.to_string();
    spawn_blocking(move || {
        Confirm::new()
            .with_prompt(q.as_str())
            .default(default)
            .interact()
            .map_err(anyhow::Error::from)
    })
}

/// Run the passed Command, capturing standard output while also passing standard output
/// it to the calling process' standard output.  Standard input and standard error, as
/// configured in the Command, are unaffected.
/// Returns a tuple of (captured output, Result<ExitStatus, io::Error>).  Caller is
/// responsible for checking that the result is OK, and that the OK value is .success().
/// If print_stdout is true, then standard output is printed as it happens.  If not,
/// standard output will be logged with debug level.
pub async fn run_capturing_stdout(cmd: &mut Command, print_stdout: bool) -> (String, Result<ExitStatus, Error>) {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => return ("".to_string(), Err(e)),
    };
    let (mut stdout, mut stderr) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());

    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![]));
    let stdout_buf_clone_for_me = stdout_buf.clone();

    // We'll read byte by byte, and write byte by byte.  This effectively "unbuffers" the
    // input coming from the child process.
    //
    // Now let's read from standard output / standard error, until child is done writing to them.
    // The order in which the bytes are printed is the order in which the bytes are read by the
    // loop.  However, standard output is line-buffered, so we do the same.  By definition,
    // this job is somewhat racy, so children that print unfinished lines to both standard
    // error and standard output *may* get their outputs read in the wrong order, and therefore
    // printed by us in the wrong order.  However, for the purposes of *capturing* output, this
    // yields the same captured output as e.g. redirecting to a file and reading from the file.
    let loops: Vec<BoxFuture<Result<(), Error>>> = vec![
        BoxFuture::boxed(Box::pin(async move {
            let mut stdout_buf = stdout_buf.lock().await;
            let mut just_read: Vec<u8> = Vec::with_capacity(65536);
            let mut our_stdout = tokio::io::stdout();
            loop {
                match stdout.read_buf(&mut just_read).await {
                    Ok(0) => break,
                    Err(e) => return Err(e),
                    Ok(_) => {
                        // Ignore any potential short or bad write.
                        if print_stdout {
                            let _ = our_stdout.write_all(&just_read).await;
                        }
                        stdout_buf.append(just_read.as_mut());
                    }
                }
            }
            Ok(())
        })),
        BoxFuture::boxed(Box::pin(async move {
            let mut just_read: Vec<u8> = Vec::with_capacity(65536);
            let mut our_stderr = tokio::io::stderr();
            loop {
                match stderr.read_buf(&mut just_read).await {
                    Ok(0) => break,
                    Err(e) => return Err(e),
                    Ok(_) => {
                        // Ignore any potential short or bad write.
                        let _ = our_stderr.write_all(&just_read).await;
                        just_read.clear();
                    }
                }
            }
            Ok(())
        })),
    ];

    let results = join_all(loops.into_iter()).await;
    let buf = stdout_buf_clone_for_me.lock().await;
    let captured_output = String::from_utf8_lossy(&buf).to_string();
    if !print_stdout {
        debug!("Standard output of command:\n{}", captured_output.yellow());
    }
    for result in results.into_iter() {
        if let Err(e) = result {
            return (captured_output, Err(e));
        }
    }

    (captured_output, child.wait().await)
}
