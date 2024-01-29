use std::{io::{self, Read}, path::Path};
use std::sync::{Arc, Mutex};
use futures_lite::StreamExt;
use flate2::read::GzDecoder;
use tar::Archive;

pub async fn download_and_extract(url: &str, output_target_dir: &Path) -> io::Result<()> {
    let client = reqwest::Client::new();
    let output_dir = Arc::new(Mutex::new(output_target_dir.to_path_buf()));
    let response;

    match client.get(url).send().await {
        Ok(res) => response = res,
        Err(error) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, error));
        }
    };
    let (tx, rx) = flume::bounded(0);

    let decoder_thread = std::thread::spawn(move || {
        let input = ChannelRead::new(rx);
        let gz = GzDecoder::new(input);
        let mut archive = Archive::new(gz);
        archive.unpack(output_dir.lock().unwrap().clone()).unwrap();
    });

    if response.status() == reqwest::StatusCode::OK {
        let mut stream = response.bytes_stream();


        while let Some(item) = stream.next().await {
            let chunk = item
                .or(Err(format!("Error while downloading file")))
                .unwrap();
            tx.send_async(chunk.to_vec()).await.unwrap();
        }
        drop(tx);
    }

    tokio::task::spawn_blocking(|| decoder_thread.join())
        .await
        .unwrap()
        .unwrap();

    Ok(())
}

struct ChannelRead {
    rx: flume::Receiver<Vec<u8>>,
    current: io::Cursor<Vec<u8>>,
}

impl ChannelRead {
    fn new(rx: flume::Receiver<Vec<u8>>) -> ChannelRead {
        ChannelRead {
            rx,
            current: io::Cursor::new(vec![]),
        }
    }
}

impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            if let Ok(vec) = self.rx.recv() {
                self.current = io::Cursor::new(vec);
            }
        }
        self.current.read(buf)
    }
}