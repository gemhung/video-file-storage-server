use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use uuid::Uuid;

const ROOT: &str = "./storage";
const BUCKET: &str = "bucket_";
const BASE: u64 = 65535;
const BUCKET_SIZE: u64 = 10;
const DUPLICATE: u64 = 3; // Duplicate to 3 places
                          //
#[derive(Clone, Debug)]
pub struct Storage {
    bucket: Vec<u64>,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Storage {
    pub async fn new() -> Self {
        let mut bucket = vec![];
        let mut index = 0;
        for i in 0..BUCKET_SIZE {
            let path = std::path::Path::new(ROOT).join(BUCKET.to_string() + &i.to_string());
            tokio::fs::create_dir_all(path).await.unwrap();
            bucket.push(index);
            index += BASE / BUCKET_SIZE;
        }

        Self { bucket }
    }

    pub async fn retrieve(&self, fileid: &Uuid) -> Option<Vec<u8>> {
        let mut index = self.find_index(fileid);
        for _i in 0..DUPLICATE {
            let path = std::path::Path::new(ROOT)
                .join(BUCKET.to_string() + &index.to_string())
                .join(fileid.to_string());
            if let Ok(data) = tokio::fs::read(path).await {
                return Some(data);
            }
            index += 1;
            // Wrap around
            if index == self.bucket.len() {
                index = 0;
            }
        }

        None
    }

    pub async fn store(&mut self, fileid: &Uuid, data: &[u8]) {
        let mut index = self.find_index(fileid);
        for _i in 0..DUPLICATE {
            let path = std::path::Path::new("./storage")
                .join(BUCKET.to_string() + &index.to_string())
                .join(fileid.to_string());
            let _ = tokio::fs::write(path, data).await;
            index += 1;
            // Wrap around
            if index == self.bucket.len() {
                index = 0;
            }
        }
    }

    pub async fn delete(&mut self, fileid: &Uuid) {
        let mut index = self.find_index(fileid);
        for _i in 0..DUPLICATE {
            let path = std::path::Path::new(ROOT)
                .join(BUCKET.to_string() + &index.to_string())
                .join(fileid.to_string());
            // Don't care if error for now
            let _ = tokio::fs::remove_file(path).await;
            index += 1;
            // Wrap around
            if index == self.bucket.len() {
                index = 0;
            }
        }
    }

    fn find_index(&self, fileid: &Uuid) -> usize {
        let target = calculate_hash(fileid) % BASE;
        match self.bucket.as_slice().binary_search(&target) {
            Ok(index) => index,
            Err(index) => index - 1,
        }
    }
}
