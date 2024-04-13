FROM rust
WORKDIR /usr/local/myapp
RUN mkdir src third-party .cargo
COPY ./.cargo /usr/local/cargo
COPY ./.cargo ./.cargo
ENV RUSTUP_DIST_SERVER https://cloudfront-static.rust-lang.org
RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked trunk wasm-bindgen-cli
COPY ./Cargo.toml ./Cargo.toml
COPY ./Trunk.toml ./Trunk.toml
COPY ./third-party ./third-party
COPY ./index.html ./index.html
COPY ./src ./src
RUN cargo install --path .
RUN trunk build --release
CMD ["trunk", "serve", "--release"]