FROM rust:1.87

ENV RUST_BACKTRACE=full

RUN groupadd -g 1000 app
RUN useradd -u 1000 -ms /bin/bash -g app app

USER app

RUN cargo install watchexec-cli
