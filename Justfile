windows_target := "x86_64-pc-windows-gnu"
linux_target := "x86_64-unknown-linux-musl"
bin_name := "download-iglive"
output_dir := "bin"


default:
  @just --list

build-windows:
  cross build --release --target {{windows_target}}
  mkdir -p {{output_dir}}
  cp target/{{windows_target}}/release/{{bin_name}}.exe {{output_dir}}/{{bin_name}}-windows.exe

build-linux:
  cross build --release --target {{linux_target}}
  mkdir -p {{output_dir}}
  cp target/{{linux_target}}/release/{{bin_name}} {{output_dir}}/{{bin_name}}-linux

build: build-windows build-linux
