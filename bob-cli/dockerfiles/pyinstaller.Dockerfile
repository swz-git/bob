FROM ghcr.io/virxec/bob-base-images/python-cross:v1

# Linux git is already installed (for pip git dependencies)

COPY . /usr/src/botsrc
WORKDIR "/usr/src/botsrc"

RUN mkdir _BOB_OUT

# Install deps for windows
RUN WINEDEBUG=-all WINEPATH="Z:\\usr\\src\\win\\git\\cmd" wine ../win/uv.exe pip install -p ../win/python pyinstaller --requirement requirements.txt
# "Compile" for windows
RUN WINEDEBUG=-all WINEPATH="Z:\\usr\\src\\win\\git\\cmd" wine ../win/python/python.exe -m PyInstaller {entry_file}

RUN mv ./dist ./_BOB_OUT/x86_64-pc-windows-msvc

# For Linux, find "\\" in {entry_file} and replace with "/"
RUN sed -i 's/\\\\/\//g' {entry_file}

# Install deps for linux
RUN UV_PYTHON="/usr/src/linux/python" ../linux/uv pip install pyinstaller --requirement requirements.txt
# "Compile" for linux
RUN ../linux/python/bin/python -m PyInstaller {entry_file}

RUN mv ./dist ./_BOB_OUT/x86_64-unknown-linux-gnu

CMD ["/bin/bash", "-c", "cd _BOB_OUT && bsdtar -cf - *"]
