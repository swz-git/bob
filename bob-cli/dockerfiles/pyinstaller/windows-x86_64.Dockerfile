FROM ghcr.io/virxec/bob-base-images/python-windows:v2

COPY . /usr/src/botsrc
WORKDIR "/usr/src/botsrc"

RUN mkdir _BOB_OUT

# Install deps (Git is already installed as well)
RUN ..\windows\uv pip install pyinstaller --requirement requirements.txt
# "Compile"
RUN ..\windows\uv run pyinstaller {entry_file}

RUN mv ./dist ./_BOB_OUT/x86_64-unknown-linux-gnu

CMD ["/bin/bash", "-c", "cd _BOB_OUT && bsdtar -cf - *"]
