use crate::error::Result;
use crate::service::IStorageService;
use futures_util::future::BoxFuture;
use std::fmt::Debug;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct FileLocalService {}

impl FileLocalService {
    pub fn new() -> Self {
        Self {}
    }
}

impl IStorageService for FileLocalService {
    fn upload(&self, name: String, data: Vec<u8>) -> BoxFuture<Result<String>> {
        let name = PathBuf::from(name);
        Box::pin(async move {
            if let Some(parent) = name.parent() {
                tokio::fs::create_dir_all(&parent).await?;
            }
            let mut f = tokio::fs::File::create(&name).await?;
            f.write(&data).await?;
            f.flush().await?;
            Ok(name.to_str().unwrap_or_default().to_string())
        })
    }

    fn download(&self, name: String) -> BoxFuture<Result<Vec<u8>>> {
        let name = PathBuf::from(name);
        Box::pin(async move {
            if let Some(parent) = name.parent() {
                tokio::fs::create_dir_all(&parent).await?;
            }
            let mut f = tokio::fs::File::open(&name).await?;
            let mut data = Vec::new();
            f.read_to_end(&mut data).await?;
            Ok(data)
        })
    }

    fn list(&self, name: String) -> BoxFuture<Result<Vec<String>>> {
        let name = PathBuf::from(name);
        Box::pin(async move {
            let mut rd = tokio::fs::read_dir(&name).await?;
            let mut result = Vec::new();
            while let Ok(v) = rd.next_entry().await {
                if let Some(v) = v {
                    result.push(v.path().display().to_string());
                } else {
                    break;
                }
            }
            Ok(result)
        })
    }

    fn remove(&self, name: String) -> BoxFuture<Result<()>> {
        let name = PathBuf::from(name);
        Box::pin(async move {
            let f = tokio::fs::remove_file(&name).await?;
            Ok(f)
        })
    }
}
