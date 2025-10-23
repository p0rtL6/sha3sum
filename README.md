# sha3sum

A Rust implementation of the sha3 algorithm.

## Building
```
cargo build --release
```

## Usage
> `-m` is optional, default mode is 224.
```
sha3sum -m <224, 256, 384, 512> [files]
```