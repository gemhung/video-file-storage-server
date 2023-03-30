FROM rust:1.67 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --locked --path .
RUN pwd && ls -al

FROM debian:bullseye-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
RUN apt-get update
WORKDIR /usr/local/bin/
RUN pwd
COPY --from=builder /usr/src/myapp/target/release/storage-server /usr/local/bin/myapp
CMD ["myapp"]
