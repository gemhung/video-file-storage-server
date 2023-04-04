use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use uuid::Uuid;

const ROOT: &str = "./storage";
const BUCKET: &str = "bucket_";
const BASE: u64 = 65535; // Magic number
const BUCKET_SIZE: usize = 10;
const DUPLICATE: u64 = 3; // Duplicate to 3 places
                          //
#[derive(Clone, Debug)]
pub struct Storage {
    bucket: Vec<(u64, usize)>, // (0,0), (6553, 1), (65535*2, 2), .... (65535*n, n)
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl Storage {
    pub async fn new() -> Self {
        // Validation 1
        if BUCKET_SIZE == 0 {
            panic!("BUCKET SIZE cannot be 0");
        }
        // Validation 2
        if DUPLICATE == 0 {
            panic!("DUPLICATE SIZE cannot be 0");
        }
        // Validation 3
        if BUCKET_SIZE > BASE as usize {
            panic!("BUCKET SIZE shouldn't be larger than BASE");
        }
        // Validation 4
        if DUPLICATE > BUCKET_SIZE as u64 {
            panic!("Duplicate size shouldn't be larger than bucket size");
        }
        let mut bucket = vec![];
        let mut val = 0;
        for i in 0..BUCKET_SIZE {
            let path = std::path::Path::new(ROOT).join(BUCKET.to_string() + &i.to_string());
            tokio::fs::create_dir_all(path).await.unwrap();
            bucket.push((val, i));
            val += BASE / BUCKET_SIZE as u64;
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
            // Next bucket cause we cannot get file from the first place
            index += 1;
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
            // Next bucket
            index += 1;
            // Wrap around
            if index == self.bucket.len() {
                index = 0;
            }
        }
    }

    pub async fn delete(&mut self, fileid: &Uuid) {
        // Find corresponding index
        let mut index = self.find_index(fileid);
        for _i in 0..DUPLICATE {
            // Construct Path
            let path = std::path::Path::new(ROOT)
                .join(BUCKET.to_string() + &index.to_string())
                .join(fileid.to_string());
            // Delete file and don't care if error for now
            let _ = tokio::fs::remove_file(path).await;
            // Next bucket
            index += 1;
            // Wrap around
            if index == self.bucket.len() {
                index = 0;
            }
        }
    }

    // O(nlogn)
    fn find_index(&self, fileid: &Uuid) -> usize {
        let target = calculate_hash(fileid) % BASE;
        let index = match self
            .bucket
            .as_slice()
            .binary_search_by_key(&target, |&(val, _index)| val)
        {
            Ok(inner) => inner,
            Err(inner) => inner - 1,
        };

        if index == self.bucket.len() {
            panic!(
                "Buckets might be exhausted, index = {}, bucket = {}",
                index,
                self.bucket.len()
            );
        }
        self.bucket[index].1
    }
}
