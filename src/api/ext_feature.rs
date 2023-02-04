use super::files::File;
use super::files::UploadedFile;
use std::collections::BinaryHeap;

pub async fn top_10_downloads(fileapi: &super::files::FilesApi) -> Vec<UploadedFile> {
    //  Method 1: min-heap
    let mut pq = BinaryHeap::new();
    let resource = fileapi.rwlock.read().await;

    // Top k frequent items algo
    for (id, file) in resource.files.iter() {
        pq.push((std::cmp::Reverse(file.download_cnt), *id));
        if pq.len() > 10 {
            pq.pop();
        }
    }

    // Simple mapping
    let mut ret = vec![];
    while let Some((_cnt, id)) = pq.pop() {
        if let Some(r) = resource.files.get(&id).map(
            |File {
                 name,
                 created_at,
                 size,
                 ..
             }| UploadedFile {
                fileid: id.to_string(),
                name: name.to_string(),
                size: *size,
                created_at: created_at.to_string(),
            },
        ) {
            ret.push(r);
        }
    }

    // Method 2 : quick select
    // Todo

    ret
}
