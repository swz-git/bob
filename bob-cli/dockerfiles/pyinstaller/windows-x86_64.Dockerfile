FROM ghcr.io/virxec/bob-base-images/python-windows:v2

COPY . "C:\\usr\\src\\botsrc"
WORKDIR "C:\\usr\\src\\botsrc"

RUN mkdir _BOB_OUT

# Install deps (Git is already installed as well)
RUN ..\windows\uv.exe pip install pyinstaller --requirement requirements.txt
# "Compile"
RUN ..\windows\uv.exe run pyinstaller {entry_file}

RUN move "dist" "_BOB_OUT\\x86_64-pc-windows-msvc"

CMD ["cmd", "/S", "/C", "cd _BOB_OUT && tar -cf - *"]
