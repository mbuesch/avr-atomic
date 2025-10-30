# Fast atomic load/store without IRQ-disable for AVR

A fast atomic type for 8-bit values on AVR microcontrollers.
It is designed to be efficient by avoiding IRQ-disable/restore overhead.

This crate provides a simple and fast way to perform atomic load and store operations on 8-bit values:
`u8`, `i8`, `bool` and any other user defined type that can be converted to and from `u8`.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
avr-atomic = "1"
```

## Example

```rust
use avr_atomic::AvrAtomic;

static VALUE_U8: AvrAtomic<u8> = AvrAtomic::new();
static VALUE_I8: AvrAtomic<i8> = AvrAtomic::new();
static VALUE_BOOL: AvrAtomic<bool> = AvrAtomic::new();

fn foo() {
    VALUE_U8.set(0x42);
    let value = VALUE_U8.get();

    VALUE_I8.set(-42);
    let value = VALUE_I8.get();

    VALUE_BOOL.set(true);
    let value = VALUE_BOOL.get();
}
```

## License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
