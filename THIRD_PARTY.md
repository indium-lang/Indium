# Third-party notices

The Rust dependency versions are recorded in Cargo.lock. Each dependency keeps
its own license; no dependency's license transfers ownership of Indium's
original code to its authors.

Direct dependencies:

- rust-sdl2 and sdl2-sys: MIT; bindings to SDL2.
- SDL2 itself: zlib license (supplied by the system or by the user on Windows).
- image: MIT OR Apache-2.0.
- fontdue: MIT OR Apache-2.0 OR Zlib.
- Symphonia: MPL-2.0. It is used as an unmodified dependency, not copied into
  this project's original Rust source.

The bundled DejaVu Sans font is supplied with the full license text at
assets/DejaVu-LICENSE.txt. That notice also covers the upstream Bitstream font
license. Keep it with redistributed font bytes.

examples/assets/README.md describes the original CC0 sound/image fixtures.
The full Cargo metadata and locked dependency source URLs are available using
`cargo metadata --locked`. No third-party dependency sources are vendored in
this source snapshot.
