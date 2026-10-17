#!/usr/bin/env bash
# Cross-compile hygiene.exe (Windows) from Linux/WSL.
#
# Preferred setup: system mingw-w64 toolchain
#   sudo apt install mingw-w64
#   rustup target add x86_64-pc-windows-gnu
#   ./build.sh
#
# No-sudo fallback: rust-lld plus a user-local copy of the mingw import
# libraries / CRT objects / libgcc (point HYGIENE_W64 at its prefix).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET=x86_64-pc-windows-gnu

die() { echo "error: $*" >&2; exit 1; }

command -v cargo >/dev/null || die "cargo not found (https://rustup.rs)"
rustup target add "$TARGET" >/dev/null 2>&1 || true

if command -v x86_64-w64-mingw32-gcc >/dev/null; then
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
else
  W64="${HYGIENE_W64:-$HOME/w64}"
  SYSROOT="$(rustc --print sysroot)"
  HOST="$(rustc -vV | sed -n 's/^host: //p')"
  RUST_LLD="$SYSROOT/lib/rustlib/$HOST/bin/rust-lld"
  MINGW_LIB="$W64/usr/x86_64-w64-mingw32/lib"

  [ -x "$RUST_LLD" ] || die "rust-lld not found at $RUST_LLD"
  [ -d "$MINGW_LIB" ] || die "no mingw-w64 gcc on PATH and mingw libs not found at $MINGW_LIB
       (install mingw-w64, or set HYGIENE_W64 to a user-local mingw prefix)"

  # Debian-style layout: usr/lib/gcc/<triple>/<version>/
  GCCDIR="$(ls -d "$W64"/usr/lib/gcc/*/*/ 2>/dev/null | head -n1 || true)"
  GCCDIR="${GCCDIR%/}"
  [ -n "$GCCDIR" ] || die "libgcc not found under $W64/usr/lib/gcc/<triple>/<version>/"

  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$RUST_LLD"
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-Lnative=$MINGW_LIB -Lnative=$GCCDIR -Clink-arg=$MINGW_LIB/crt2.o -Clink-arg=$MINGW_LIB/crtbegin.o -Clink-arg=$MINGW_LIB/crtend.o"
fi

cd "$HERE"
cargo build --release --target "$TARGET"

echo
echo "built: $HERE/target/$TARGET/release/hygiene.exe"
