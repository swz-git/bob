FROM mcr.microsoft.com/dotnet/sdk:10.0

RUN apt-get update && apt-get install -y --no-install-recommends \
    lld clang zlib1g-dev libkrb5-dev \
    mingw-w64 \
    && rm -rf /var/lib/apt/lists/*

# Install xwin which "Allows downloading and repacking the MSVC CRT and Windows SDK for cross compilation"
RUN curl -L https://github.com/Jake-Shadle/xwin/releases/download/0.8.0/xwin-0.8.0-x86_64-unknown-linux-musl.tar.gz \
    | tar -xz -C /tmp \
    && mv /tmp/xwin-0.8.0-x86_64-unknown-linux-musl/xwin /usr/local/bin/xwin

WORKDIR "/usr/src"
COPY . .

WORKDIR "/usr/src/{base_dir}"
RUN dotnet publish -r linux-x64 -c Release -p:DebugType=None -p:DebugSymbols=false -p:PublishAot=true -o /usr/src/_BOB_OUT/x86_64-linux

# PublishAotCrossXWin uses xwin to enable cross compilation
RUN dotnet add package PublishAotCrossXWin --version 1.2.0
RUN dotnet publish -r win-x64 -c Release -p:DebugType=None -p:DebugSymbols=false -p:PublishAot=true -p:AcceptVSBuildToolsLicense=true -o /usr/src/_BOB_OUT/x86_64-windows

CMD ["/bin/bash", "-c", "cd /usr/src/_BOB_OUT && tar -cf - ./*"]
