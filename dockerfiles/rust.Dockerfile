FROM rust:1.81-slim

# Allows building to ...-windows-gnu targets
RUN apt update && apt install -y mingw-w64

RUN ["rustup", "component", "add", "rustfmt"]
# {{ for target in targets }}
RUN ["rustup", "target", "add", "{target}"]
# {{ endfor }}

WORKDIR "/usr/src"
COPY . .

RUN rm -rf target

# {{ for target in targets }}
RUN cargo build -r --target {target} --bin {bin_name}
# {{ endfor }}
RUN mkdir _binaries
RUN mv ./target/**/release/{bin_name} ./target/**/release/{bin_name}.exe ./_binaries


CMD ["/bin/bash", "-c", "cd _binaries && tar -cf - ./*"]
