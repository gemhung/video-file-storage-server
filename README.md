<h1 align="center">Restful Video Storage Server</h1>

<div align="center">
  <!-- CI -->
  <img src="https://github.com/cityos-dev/Gembright-Stone-Hung/actions/workflows/action.yaml/badge.svg" />
  <img src="https://github.com/cityos-dev/Gembright-Stone-Hung/actions/workflows/clippy.yaml/badge.svg" />
  <a href="https://github.com/rust-secure-code/safety-dance/">
    <img src="https://img.shields.io/badge/unsafe-forbidden-success.svg?style=flat-square"
      alt="Unsafe Rust forbidden" />
  </a>
  <a href="https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html">
    <img src="https://img.shields.io/badge/rustc-1.65.0+-ab6000.svg"
      alt="rustc 1.65.0+" />
  </a>
</div>

# keywords
```
poem, openapi, tokio, tracing, storage, consisten hashing, file processing, http, uuid, restful api
```
# Swagger UI
  * Check out [http://0.0.0.0:8080] 

# Web framework (Why poem ?) 
  * There are many excellent web frameworks such as `actix-web`, `axum` or `rocket` but might be way too powerful to quickly get familiar with 
  * The simple reason to pick up `poem` is because I feel like it has a good support to `openapi` and can help me build the solution quickly
  * It's also a good chance to showcase how I pick up a new framework and how to get familiar with it
  * The implementation focuses more on rust coding to make it neat and clean

# Storage
  * For now, we store uploaded files into local hard disk. In reality, they should be stored on cloud storage such as aws s3
  * Under `./storage` dir, it created 10 buckets dir to simulate balancing workload
  * In reality, some bucket maybe gone accidently and stored files will be gone, too
  * To prevent from the accident above, we can duplicate uploaded file to the next 3 buckes
  * I implemented the storage class with basic consisten hashing because it's useful to deal with accidents above
  * For now, my implementation didn't have rebalance function yet but it uses binary search to get the index where data was stored
  * For more info about `consisten hashing`, see [https://en.wikipedia.org/wiki/Consistent_hashing]

# Resource
  * We use `rwlock` to protect resource data for now. The other way is to use `mpsc` channel
  * `Rwlock` has a better fine grained access but also introduce more complexity and it's easier to have deadlock

# Uuid
  * For security reason, I feel like uuid is a good choice as our stored file name and then create a separate mapping in separated meta data

# Rate-limiter
  * I feel like it's common to have rate-limiter to avoid from too many request
  * For now, the soluition uses a middleware to support rate-limiter for `1000 queries` in `30 seconds`

# File size limit
  * I feel like it's common to limit the file size for uploading a file
  * For now, the solution limits file size up to `1GB`

# Ext-solutions
  - Top 10 downloaded files
    * I feel like it's common for users to know what the top 10 downloaded files are
    * Take a look at `ext_feature.rs` for `fn top_10_downloads(...)` 
  - The generated openapi spec [http://0.0.0.0:8080/spec] or [http://0.0.0.0:8080/spec_yaml] for file

# Todo
  * Storage
    - Add rebalance function for my `consisten hashing` storage
  * X-Api-Key
    - It's common to have x-api-key authentication
  * Meta data
    - For scalability, the file meta data can use `redis` to save data rather than holding it in memory
  * Cache
    - Is it good to cache the top 10 downloaded files in terms of performance ?
  * Recovery
    - Because application may relaunch, we should recover meta data
  *  Cache
    - Maybe it's nice that we cache files data in memory for the top 10 downloaded files
  * Rate limiter
    - For now, the rate limiter running in each task is independent. If we'd like to have an universal rate-limiter for all tasks running. We might have to implement it in `redis` (See [https://developer.redis.com/develop/dotnet/aspnetcore/rate-limiting/sliding-window/])
  * Magic number
    - Parameterize magic numbers which can be found from rate-limiter, maximum file size upload and host address/port
  * Test
    - Missing a lot of test cases for now
  * Error handling
    - It's nice to have a more detailed, well-defined customized error class
  
