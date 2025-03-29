FROM rust:1.81-slim

# Allows building for ...-windows-gnu targets
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
RUN mkdir _BOB_OUT

# {{ for target in targets }}
RUN mkdir _BOB_OUT/{target}
RUN mv ./target/{target}/release/{bin_name}* ./_BOB_OUT/{target}/
# {{ endfor }}



CMD ["/bin/bash", "-c", "cd _BOB_OUT && tar -cf - ./*"]
