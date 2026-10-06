# kpack

Delta-compression engine for immutable patching and image assembly.

`kpack` applies a small, explicit instruction set to rebuild a buffer from a
payload and, when needed, a reference image. It is `no_std`, allocation-free,
and has no dependencies. Intended for firmware images, immutable archives, and
other environments where the patch format must stay auditable.

This crate is not related to the Kubernetes [kpack](https://github.com/buildpacks-community/kpack) build service.

## Install

```toml
[dependencies]
kpack = "0.1"
```

```sh
cargo add kpack
```

## Example

Rebuild a 64-byte image from five instructions. `execute` never allocates and
never reads past the caller-provided buffers.

```rust
use kpack::{execute, Opcode};

let mut image = [0u8; 64];

execute(&Opcode::Lit, 10, b"AMENTYS-OS", &mut image[0..10], None);
execute(&Opcode::Rle, 20, &[0xAA], &mut image[10..30], None);

let dictionary = b"COREDATABOOT";
execute(&Opcode::Dict, 4, &[2, 0, 1], &mut image[30..42], Some(dictionary));

execute(
    &Opcode::Xor,
    0x42,
    &[0x11, 0x07, 0x01, 0x17, 0x10, 0x0B, 0x16, 0x1B],
    &mut image[42..50],
    None,
);

let parent = b"AmentysOSBase!";
let delta = [
    0x01, 0x00, 0x00, 0x09, 0x00, // copy 9 bytes from parent offset 0
    0x02, 0x04, 0x00, b'C', b'o', b'r', b'e', // insert "Core"
    0x01, 0x0D, 0x00, 0x01, 0x00, // copy '!' from parent offset 13
    0x00,
];
execute(&Opcode::Delta, 0, &delta, &mut image[50..64], Some(parent));

assert_eq!(&image[0..10], b"AMENTYS-OS");
assert_eq!(&image[10..30], &[0xAA; 20]);
assert_eq!(&image[30..42], b"BOOTCOREDATA");
assert_eq!(&image[42..50], b"SECURITY");
assert_eq!(&image[50..64], b"AmentysOSCore!");
```

## Instructions

`Opcode` is `#[repr(u8)]`. Decode a wire byte with `Opcode::from_u8`.

| Opcode | Value | `param` | Payload | Reference |
| --- | --- | --- | --- | --- |
| `Lit` | `0` | byte count | raw bytes | ignored |
| `Delta` | `1` | ignored | copy/insert program | parent image |
| `Rle` | `2` | repeat count | first byte is the fill value | ignored |
| `Seed` | `3` | xorshift seed (`0` becomes `1`) | sparse patches | ignored |
| `Dict` | `4` | word size in bytes | one index byte per word | dictionary |
| `Xor` | `5` | low 8 bits are the key | input bytes | optional key stream |
| `Rs` | `6` | shift, masked to `0..=7` | input bytes | ignored |

`execute` dispatches to `execute_lit`, `execute_delta`, `execute_rle`,
`execute_seed`, `execute_dict`, `execute_xor`, and `execute_rs`. Lengths are
clamped to the output buffer. A missing reference zeroes the output for
`Delta` and `Dict`.

### Delta program

Instructions are consumed in order. `0x00` ends the program.

- `0x01`, `offset: u16` LE, `size: u16` LE — copy `size` bytes from the reference at `offset`.
- `0x02`, `size: u16` LE, then `size` bytes — insert those bytes.

### Seed program

The output is filled with an xorshift32 stream (`state ^= state << 13`,
`state ^= state >> 17`, `state ^= state << 5`, low byte kept). The payload
then overlays sparse bytes: `count: u16` LE, followed by `count` records of
`pos: u16` LE and `value: u8`.

### Dictionary

`param` is the word size. Each payload byte is an index. Word `i` is the slice
`reference[i * word_size ..]`. An out-of-range index writes zeroes.

## Features

- `no_std` and `no_alloc`. Tests are the only `std` path.
- No dependencies.
- Bounded writes: every op stops at `output_buffer.len()`.
- Opcode set is a closed `u8` enum, so a patch stream can be audited without a decompressor.

## Documentation

API docs are on [docs.rs/kpack](https://docs.rs/kpack).

## License

[AGPL-3.0-or-later](LICENSE).
