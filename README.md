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
poem, poem-openapi, tokio, tracing, storage, file processing, http, uuid
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
  * That's where `consisten hashing` comes to the picture
  * For now, I implemented the storage class with basic consisten hashing which didn't have rebalance function
  * Though my implementation didn't have rebalance function, it still use binary search to get the index where data was stored
  * For more info about `consisten hashing`, see [https://en.wikipedia.org/wiki/Consistent_hashing]

# Resource
  * We use `rwlock` to protect resource data for now. The other way is to use `mpsc` channel
  * `Rwlock` has a better fine grained access but also intruduce more complexity and easier to have deadlock

# Uuid
  * For security reason, I feel like using uuid as file name when saving the file and create a separate mapping in separated meta data

# Rate-limiter
  * I feel like it's common to have rate-limiter to avoid from too many request
  * The soluition uses a middle to support rate-limiter for `1000 queries` in `30 seconds` for now

# File size limit
  * I feel like it's common to limit the file size for uploading a file
  * The solution limits file size up to `1GB` for now

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
  * Recovery
    - Because application may relaunch, we should recover meta data
  *  Cache
    - Maybe it's nice that we cache files data in memory for the top 10 downloaded files
  * Rate limiter
    - For now, the rate limiter running in each task is independent. If we'd like to have an universal rate-limiter for all tasks running. We might have to implement it in `redis` (See [https://developer.redis.com/develop/dotnet/aspnetcore/rate-limiting/sliding-window/])
  * Magic number
    - Parameterize magic numbers which can be found from rate-limiter, maximum file size upload and host address/port
  * Test
    - Missing a lot of testings for now
  * Error handling
    - It's nice to have a more detailed, well-defined customized error class
  

# Challenge Statement

This challenge is about creating a simple video storage server with REST APIs

## Details

You are tasked to develop a simple video storage server with REST APIs, which should have:
- **CRUD implemention** as described in the [Open API definition](./api.yaml). (This document only contains the minimum and may need to be added).
- **Dockerfile** and **docker-compose** to build and run the server and other required services as docker containers.
- The endpoints of the server are exposed to the host machine.

## What we expect

When working on this challenge, be sure to:

- prove correctness of the code. We don't expect 100% test coverage but highlights on critical paths and logic is very welcome.
  
- handle errors and bad inputs
  
- provide user friendliness of installation and setup. We'll run `docker-compose up` in a clean environment without toolchains, JVM or SDKs and expect to see a server and the needed containers building and starting (this includes DB and all the other images used to complete the task).

We understand taking a challenge is time consuming, so feel free to choose an additional feature you feel passionate about and explain in words how you would like to implement it. We can discuss it further during the next interview steps!
See the [Bonus point and extensions](#bonus-points-and-extensions) section.
  

## How to submit your solution

- Push your code to this repository in the `main` branch.
- If you choose to implement one of the "bonus" features, please do so in a separate branch named `ext-solution`.
- Make sure the endpoints follow the path suggested in the `api.yaml` file (v1 included!).
- If your setup is correct the CI will return a green check and you can move forward. 

⚠️ **Note**: the CI/CD runs checks against a set of tests necessary to consider the assigment correct. _Without a green check we won't review the challenge_ as we can safely assume the overall solution is incomplete. Also, please *DO NOT* change the CI/CD workflow file _or_ the `test/tester.json` file - if you want to add your own tests, please add them in a dedicated folder of your choice.

*Note*

If you add or change APIs, include its OpenAPI document. However, please note that your server may be accessed by external clients in accordance with the given OpenAPI document and automated tests will hit the endpoints as described in [api.yaml](./api.yaml), therefore any change in the base path could result in 404 or false negative.

