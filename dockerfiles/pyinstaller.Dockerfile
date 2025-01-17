FROM debian:bookworm-slim
RUN dpkg --add-architecture i386 && apt update
RUN apt install -y wget
RUN mkdir -pm755 /etc/apt/keyrings
RUN wget -O /etc/apt/keyrings/winehq-archive.key https://dl.winehq.org/wine-builds/winehq.key
RUN wget -NP /etc/apt/sources.list.d/ https://dl.winehq.org/wine-builds/debian/dists/bookworm/winehq-bookworm.sources
RUN apt update
RUN apt install --install-recommends -y winehq-staging
RUN apt install -y git libarchive-tools binutils

# Windows prep
WORKDIR "/usr/src/win"

# Install windows python
RUN wget https://github.com/astral-sh/python-build-standalone/releases/download/20250115/cpython-3.12.8+20250115-x86_64-pc-windows-msvc-install_only_stripped.tar.gz -O python.tar.gz
RUN bsdtar -xf python.tar.gz

# Install windows uv
RUN wget https://github.com/astral-sh/uv/releases/download/0.5.6/uv-x86_64-pc-windows-msvc.zip -O uv.zip
RUN bsdtar -xf uv.zip

# Install windows git (for pip git dependencies)
RUN wget https://github.com/git-for-windows/git/releases/download/v2.47.1.windows.1/MinGit-2.47.1-64-bit.zip -O git.zip
RUN mkdir git
RUN bsdtar -xf git.zip --directory git

# Linux prep
WORKDIR "/usr/src/linux"

# Install linux python
RUN wget https://github.com/astral-sh/python-build-standalone/releases/download/20250115/cpython-3.12.8+20250115-x86_64_v2-unknown-linux-gnu-install_only_stripped.tar.gz -O python.tar.gz
RUN bsdtar -xf python.tar.gz

# Install linux uv
RUN wget https://github.com/astral-sh/uv/releases/download/0.5.6/uv-x86_64-unknown-linux-gnu.tar.gz -O uv.tar.gz
RUN bsdtar -xf uv.tar.gz
RUN mv uv-*/** .

# Linux git is already installed (for pip git dependencies)

COPY . /usr/src/botsrc
WORKDIR "/usr/src/botsrc"

RUN mkdir _BOB_OUT

# Install deps for windows
RUN WINEDEBUG=-all WINEPATH="Z:\\usr\\src\\win\\git\\cmd" wine ../win/uv.exe pip install -p ../win/python pyinstaller --requirement requirements.txt
# "Compile" for windows
RUN WINEDEBUG=-all WINEPATH="Z:\\usr\\src\\win\\git\\cmd" wine ../win/python/python.exe -m PyInstaller {entry_file}

RUN mv ./dist ./_BOB_OUT/x86_64-pc-windows-msvc

# Install deps for linux
RUN UV_PYTHON="/usr/src/linux/python" ../linux/uv pip install pyinstaller --requirement requirements.txt
# "Compile" for linux
RUN ../linux/python/bin/python -m PyInstaller {entry_file}

RUN mv ./dist ./_BOB_OUT/x86_64-unknown-linux-gnu

CMD ["/bin/bash", "-c", "cd _BOB_OUT && bsdtar -cf - *"]
