# Bob Dockerfile

All dockerfiles shall output a tar file to stdout with the following file
structure `/[PLATFORM]/platform_specific_files`. [PLATFORM] ideally being a LLVM
target triple, but only needs to contain the name of the OS (`linux`/`windows`).

## Example file structure of the tar file

* /
  * x86_64-unknown-linux-gnu/
    * bot
    * libcool_library.so
  * x86_64-pc-windows-msvc/
    * bot.exe
    * cool_library.dll
