#![allow(async_fn_in_trait)]
use std::{fs::File, io::Write, path::Path};

use futures::{future::BoxFuture, stream, StreamExt};
use ic_management_types::Artifact;
use itertools::Itertools;
use log::{info, warn};
use mockall::automock;
use reqwest::StatusCode;
use sha2::{Digest, Sha256};

pub struct ArtifactDownloaderImpl {}

#[automock]
// If we try to fix this lint, automock complains
#[allow(elided_named_lifetimes)]
pub trait ArtifactDownloader: Sync + Send {
    fn get_s3_cdn_image_url<'a>(&'a self, version: &'a str, s3_subdir: &'a str) -> String {
        format!(
            "https://download.dfinity.systems/ic/{}/{}/update-img/update-img.tar.zst",
            version, s3_subdir
        )
    }

    fn get_r2_cdn_image_url<'a>(&'a self, version: &'a str, s3_subdir: &'a str) -> String {
        format!(
            "https://download.dfinity.network/ic/{}/{}/update-img/update-img.tar.zst",
            version, s3_subdir
        )
    }

    fn download_file_and_get_sha256<'a>(&'a self, download_url: &'a str) -> BoxFuture<'_, anyhow::Result<String>> {
        Box::pin(async move {
            let url = url::Url::parse(download_url)?;
            let subdir = format!("{}{}", url.domain().expect("url.domain() is None"), url.path().to_owned());
            // replace special characters in subdir with _
            let subdir = subdir.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
            let download_dir = format!("{}/tmp/ic/{}", dirs::home_dir().expect("home_dir is not set").as_path().display(), subdir);
            let download_dir = Path::new(&download_dir);

            std::fs::create_dir_all(download_dir).unwrap_or_else(|_| panic!("create_dir_all failed for {}", download_dir.display()));

            let download_image = format!("{}/update-img.tar.gz", download_dir.to_str().unwrap());
            let download_image = Path::new(&download_image);

            let response = reqwest::get(download_url).await?;

            if response.status() != StatusCode::RANGE_NOT_SATISFIABLE && !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Download failed with http_code {} for {}",
                    response.status(),
                    download_url
                ));
            }
            info!("Download {} succeeded {}", download_url, response.status());

            let mut file = match File::create(download_image) {
                Ok(file) => file,
                Err(err) => return Err(anyhow::anyhow!("Couldn't create a file: {}", err)),
            };

            let content = response.bytes().await?;
            file.write_all(&content)?;

            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = hasher.finalize();
            let stringified_hash = hash[..].iter().map(|byte| format!("{:01$x?}", byte, 2)).collect::<Vec<String>>().join("");
            info!("File saved at {} has sha256 {}", download_image.display(), stringified_hash);
            Ok(stringified_hash)
        })
    }

    fn download_images_and_validate_sha256<'a>(
        &'a self,
        image: &'a Artifact,
        version: &'a str,
        ignore_missing_urls: bool,
    ) -> BoxFuture<'_, anyhow::Result<(Vec<String>, String)>> {
        Box::pin(async move {
            let update_urls = vec![
                self.get_s3_cdn_image_url(version, &image.s3_folder()),
                self.get_r2_cdn_image_url(version, &image.s3_folder()),
            ];

            // Download images, verify them and compare the SHA256
            let hash_and_valid_urls: Vec<(String, &String)> = stream::iter(&update_urls)
                .filter_map(|update_url| async move {
                    match self.download_file_and_get_sha256(update_url).await {
                        Ok(hash) => {
                            info!("SHA256 of {}: {}", update_url, hash);
                            Some((hash, update_url))
                        }
                        Err(err) => {
                            warn!("Error downloading {}: {}", update_url, err);
                            None
                        }
                    }
                })
                .collect()
                .await;
            let hashes_unique = hash_and_valid_urls.iter().map(|(h, _)| h.clone()).unique().collect::<Vec<String>>();
            let expected_hash: String = match hashes_unique.len() {
                0 => {
                    return Err(anyhow::anyhow!(
                        "Unable to download the update image from none of the following URLs: {}",
                        update_urls.join(", ")
                    ))
                }
                1 => {
                    let hash = hashes_unique.into_iter().next().unwrap();
                    info!("SHA256 of all download images is: {}", hash);
                    hash
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Update images do not have the same hash: {:?}",
                        hash_and_valid_urls.iter().map(|(h, u)| format!("{}  {}", h, u)).join("\n")
                    ))
                }
            };
            let update_urls = hash_and_valid_urls.into_iter().map(|(_, u)| u.clone()).collect::<Vec<String>>();

            if update_urls.is_empty() {
                return Err(anyhow::anyhow!(
                    "Unable to download the update image from none of the following URLs: {}",
                    update_urls.join(", ")
                ));
            } else if update_urls.len() == 1 {
                if ignore_missing_urls {
                    warn!("Only 1 update image is available. At least 2 should be present in the proposal");
                } else {
                    return Err(anyhow::anyhow!(
                        "Only 1 update image is available. At least 2 should be present in the proposal"
                    ));
                }
            }
            Ok((update_urls, expected_hash))
        })
    }
}

impl ArtifactDownloader for ArtifactDownloaderImpl {}
