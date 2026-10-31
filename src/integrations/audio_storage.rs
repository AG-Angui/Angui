use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use futures_util::future::BoxFuture;
use uuid::Uuid;

/// Private audio object boundary. Business services use object keys, so an
/// S3 implementation can replace the local adapter without changing review
/// or authorization code.
pub trait AudioStorage: Send + Sync {
    fn report_key(&self, id: &str, extension: &str) -> String;
    fn recording_key(&self, space_id: &str, user_id: &str, utc_stamp: &str, id: &str) -> String;
    fn save<'a>(&'a self, key: &'a str, bytes: Vec<u8>) -> BoxFuture<'a, io::Result<()>>;
    fn read<'a>(&'a self, key: &'a str) -> BoxFuture<'a, io::Result<Vec<u8>>>;
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, io::Result<()>>;
}

pub type SharedAudioStorage = Arc<dyn AudioStorage>;

#[derive(Clone)]
pub struct LocalAudioStorage {
    root: PathBuf,
}

impl LocalAudioStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, key: &str) -> io::Result<PathBuf> {
        let path = Path::new(key);
        if key.is_empty()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid audio object key",
            ));
        }
        Ok(self.root.join(path))
    }
}

impl AudioStorage for LocalAudioStorage {
    fn report_key(&self, id: &str, extension: &str) -> String {
        format!("voice/{id}.{extension}")
    }

    fn recording_key(&self, space_id: &str, user_id: &str, utc_stamp: &str, id: &str) -> String {
        format!("{space_id}/{user_id}/{utc_stamp}-{id}.wav")
    }

    fn save<'a>(&'a self, key: &'a str, bytes: Vec<u8>) -> BoxFuture<'a, io::Result<()>> {
        let path = self.path(key);
        Box::pin(async move {
            let path = path?;
            tokio::task::spawn_blocking(move || {
                let parent = path
                    .parent()
                    .ok_or_else(|| io::Error::other("audio path has no parent"))?;
                fs::create_dir_all(parent)?;
                let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
                let result = (|| {
                    let mut options = fs::OpenOptions::new();
                    options.write(true).create_new(true);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        options.mode(0o600);
                    }
                    let mut file = options.open(&temporary)?;
                    io::Write::write_all(&mut file, &bytes)?;
                    file.sync_all()?;
                    fs::rename(&temporary, path)
                })();
                if result.is_err() {
                    let _ = fs::remove_file(temporary);
                }
                result
            })
            .await
            .map_err(io::Error::other)?
        })
    }

    fn read<'a>(&'a self, key: &'a str) -> BoxFuture<'a, io::Result<Vec<u8>>> {
        let path = self.path(key);
        Box::pin(async move {
            let path = path?;
            tokio::task::spawn_blocking(move || {
                let metadata = fs::metadata(&path)?;
                if metadata.len() > 10 * 1024 * 1024 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "audio object exceeds limit",
                    ));
                }
                fs::read(path)
            })
            .await
            .map_err(io::Error::other)?
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, io::Result<()>> {
        let path = self.path(key);
        Box::pin(async move {
            let path = path?;
            tokio::task::spawn_blocking(move || match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            })
            .await
            .map_err(io::Error::other)?
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AudioStorage, LocalAudioStorage};
    use uuid::Uuid;

    #[tokio::test]
    async fn local_storage_keeps_generated_keys_private_and_rejects_traversal() {
        let root = std::env::temp_dir().join(format!("angui-audio-storage-{}", Uuid::new_v4()));
        let storage = LocalAudioStorage::new(root.clone());
        let key = storage.recording_key("space-1", "user-1", "20260929T000000.000Z", "recording-1");
        storage
            .save(&key, b"private-audio".to_vec())
            .await
            .expect("write audio");
        assert_eq!(
            storage.read(&key).await.expect("read audio"),
            b"private-audio"
        );
        assert!(storage.save("../outside.wav", vec![1]).await.is_err());
        assert!(storage.read("/absolute.wav").await.is_err());
        storage.delete(&key).await.expect("delete audio");
        assert!(storage.read(&key).await.is_err());
        std::fs::remove_dir_all(root).expect("remove isolated temporary test directory");
    }
}
