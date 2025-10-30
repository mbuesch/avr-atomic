// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

//! # AvrAtomic
//!
//! A fast atomic type for 8-bit values on AVR microcontrollers.
//!
//! # Example
//!
//! ```
//! use avr_atomic::AvrAtomic;
//!
//! static VALUE_U8: AvrAtomic<u8> = AvrAtomic::new();
//! static VALUE_I8: AvrAtomic<i8> = AvrAtomic::new();
//! static VALUE_BOOL: AvrAtomic<bool> = AvrAtomic::new();
//!
//! assert_eq!(VALUE_U8.get(), 0);
//! VALUE_U8.set(0x42);
//! assert_eq!(VALUE_U8.get(), 0x42);
//!
//! assert_eq!(VALUE_I8.get(), 0);
//! VALUE_I8.set(-42);
//! assert_eq!(VALUE_I8.get(), -42);
//!
//! assert_eq!(VALUE_BOOL.get(), false);
//! VALUE_BOOL.set(true);
//! assert_eq!(VALUE_BOOL.get(), true);
//! ```

#![cfg_attr(target_arch = "avr", no_std)]

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{Ordering::SeqCst, fence},
};

/// Lock for Non-AVR platforms.
/// This is mainly useful for testing only.
#[cfg(not(target_arch = "avr"))]
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Low level atomic read primitive.
#[inline(always)]
unsafe fn read_atomic(ptr: *const u8) -> u8 {
    // If we are *not* on AVR, fall back to a lock.
    #[cfg(not(target_arch = "avr"))]
    let _guard = LOCK.lock();

    fence(SeqCst);

    // SAFETY: An 8 bit load is atomic on AVR.
    //
    // This code expects the compiler to emit a single LD, LDS or LDD
    // for the read_volatile.
    //
    // While this is not guaranteed as such by the compiler
    // it probably is a fair assumption that we can depend on.
    // Most C programs on AVR have similar assumptions built in.
    //
    // I want to avoid using inline assembly here, because that
    // would mean quite some overhead by the required indirect
    // memory access instead of the static direct memory access (LDS).
    let value = unsafe { ptr.read_volatile() };

    fence(SeqCst);
    value
}

/// Low level atomic write primitive.
#[inline(always)]
unsafe fn write_atomic(ptr: *mut u8, value: u8) {
    // If we are *not* on AVR, fall back to a lock.
    #[cfg(not(target_arch = "avr"))]
    let _guard = LOCK.lock();

    fence(SeqCst);

    // SAFETY: An 8 bit store is atomic on AVR.
    //
    // This code expects the compiler to emit a single ST, STS or STD
    // for the write_volatile.
    //
    // While this is not guaranteed as such by the compiler
    // it probably is a fair assumption that we can depend on.
    // Most C programs on AVR have similar assumptions built in.
    //
    // I want to avoid using inline assembly here, because that
    // would mean quite some overhead by the required indirect
    // memory access instead of the static direct memory access (STS).
    unsafe { ptr.write_volatile(value) };

    fence(SeqCst);
}

/// Trait convert to and from the raw `u8` value.
pub trait AvrAtomicConvert: Copy {
    /// Convert from `u8` to `Self`.
    fn from_u8(value: u8) -> Self;

    /// Convert from `Self` to `u8`.
    fn to_u8(self) -> u8;
}

impl AvrAtomicConvert for u8 {
    #[inline(always)]
    fn from_u8(value: u8) -> Self {
        value
    }

    #[inline(always)]
    fn to_u8(self) -> u8 {
        self
    }
}

impl AvrAtomicConvert for i8 {
    #[inline(always)]
    fn from_u8(value: u8) -> Self {
        value as _
    }

    #[inline(always)]
    fn to_u8(self) -> u8 {
        self as _
    }
}

impl AvrAtomicConvert for bool {
    #[inline(always)]
    fn from_u8(value: u8) -> Self {
        value != 0
    }

    #[inline(always)]
    fn to_u8(self) -> u8 {
        self as _
    }
}

/// A fast atomic type for 8-bit values on AVR microcontrollers.
///
/// This type has no IRQ-disable/restore or other locking overhead on AVR.
///
/// This type provides atomic load and store operations for `u8`, `i8`, and `bool`
/// by default. But you can extend the supported types with your own types by
/// implementing [AvrAtomicConvert].
///
/// # Memory Safety and Compiler Guarantees
///
/// The atomicity of this type relies on the assumption that the Rust compiler will
/// generate single, atomic instructions (`LD`, `LDS`, `LDD` for loads and `ST`,
/// `STS`, `STD` for stores) for the [core::ptr::read_volatile] and [core::ptr::write_volatile]
/// operations on a single byte.
///
/// This is a common and generally safe assumption on the AVR platform, and is
/// a practice inherited from C programming for AVRs. However, it is not a
/// strict guarantee made by the Rust compiler. A future compiler version could
/// theoretically break this assumption, though it is unlikely for this target.
///
/// This approach is a trade-off that prioritizes performance and avoids the
/// complexities and potential performance penalties of inline assembly, while
/// still providing a level of safety that is considered acceptable by many
/// embedded developers on this platform.
#[repr(transparent)]
pub struct AvrAtomic<T> {
    // Interior mutable data.
    data: UnsafeCell<u8>,
    _phantom: PhantomData<T>,
}

impl<T: AvrAtomicConvert> AvrAtomic<T> {
    /// Create a new [AvrAtomic] with the initial interior data being `0_u8`.
    #[inline(always)]
    pub const fn new() -> AvrAtomic<T> {
        Self {
            data: UnsafeCell::new(0),
            _phantom: PhantomData,
        }
    }

    /// Atomically read the current value.
    ///
    /// This atomic read is also a full SeqCst memory barrier.
    #[inline(always)]
    pub fn get(&self) -> T {
        // SAFETY: The pointer passed to `read_atomic` is a valid pointer to `u8`.
        let value = unsafe { read_atomic(self.data.get()) };
        T::from_u8(value)
    }

    /// Atomically write a new value.
    ///
    /// This atomic write is also a full SeqCst memory barrier.
    #[inline(always)]
    pub fn set(&self, value: T) {
        let value = value.to_u8();
        // SAFETY: The pointer passed to `write_atomic` is a valid pointer to `u8`.
        unsafe { write_atomic(self.data.get(), value) }
    }
}

impl<T: AvrAtomicConvert> Default for AvrAtomic<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: The atomic guarantees that `Sync` access is safe.
unsafe impl<T: Send> Sync for AvrAtomic<T> {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_u8() {
        let a: AvrAtomic<u8> = AvrAtomic::new();
        assert_eq!(a.get(), 0);
        a.set(0x5A);
        assert_eq!(a.get(), 0x5A);
        a.set(0);
        assert_eq!(a.get(), 0);
    }

    #[test]
    fn test_i8() {
        let a: AvrAtomic<i8> = AvrAtomic::new();
        assert_eq!(a.get(), 0);
        a.set(-42);
        assert_eq!(a.get(), -42);
        a.set(0);
        assert_eq!(a.get(), 0);
    }

    #[test]
    fn test_bool() {
        let a: AvrAtomic<bool> = AvrAtomic::new();
        assert_eq!(a.get(), false);
        a.set(true);
        assert_eq!(a.get(), true);
        a.set(false);
        assert_eq!(a.get(), false);
    }
}

// vim: ts=4 sw=4 expandtab
