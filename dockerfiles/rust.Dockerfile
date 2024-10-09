FROM rust:1.81-slim
RUN ["rustup", "component", "add", "rustfmt"]

WORKDIR "/usr/src"
COPY . .

RUN ["cargo", "build", "-r", "--bin={bin_name}" ]
CMD ["cat", "target/release/{bin_name}"]
