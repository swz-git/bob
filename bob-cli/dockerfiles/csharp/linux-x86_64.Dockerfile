FROM mcr.microsoft.com/dotnet/sdk:8.0

RUN apt-get update && apt-get install -y clang zlib1g-dev

COPY . "/usr/src"
WORKDIR /usr/src/

RUN cd {base_dir}
RUN dotnet publish -r linux-x64 -c Release -p:DebugType=None -p:DebugSymbols=false -p:PublishAot=true -o /usr/src/_BOB_OUT/x86_64-linux

CMD ["/bin/bash", "-c", "cd /usr/src/_BOB_OUT && tar -cf - ./*"]
