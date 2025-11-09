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
//! assert_eq!(VALUE_U8.load(), 0);
//! VALUE_U8.store(0x42);
//! assert_eq!(VALUE_U8.load(), 0x42);
//!
//! assert_eq!(VALUE_I8.load(), 0);
//! VALUE_I8.store(-42);
//! assert_eq!(VALUE_I8.load(), -42);
//!
//! assert_eq!(VALUE_BOOL.load(), false);
//! VALUE_BOOL.store(true);
//! assert_eq!(VALUE_BOOL.load(), true);
//! ```
//!
//! # Implement AvrAtomic for your own type
//!
//! ```
//! use avr_atomic::{AvrAtomic, AvrAtomicConvert};
//!
//! #[derive(Copy, Clone)]
//! struct MyFoo {
//!     inner: u8,
//! }
//!
//! impl AvrAtomicConvert for MyFoo {
//!     fn from_u8(value: u8) -> Self {
//!         Self { inner: value }
//!     }
//!
//!     fn to_u8(self) -> u8 {
//!         self.inner
//!     }
//! }
//!
//! static VALUE: AvrAtomic<MyFoo> = AvrAtomic::new();
//!
//! assert_eq!(VALUE.load().inner, 0);
//! VALUE.store(MyFoo { inner: 2 } );
//! assert_eq!(VALUE.load().inner, 2);
//! ```

#![cfg_attr(target_arch = "avr", no_std)]
#![cfg_attr(target_arch = "avr", feature(asm_experimental_arch))]

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{Ordering::SeqCst, fence},
};

/// Lock for Non-AVR platforms.
/// This is mainly useful for testing only.
#[cfg(not(target_arch = "avr"))]
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(target_arch = "avr")]
#[inline(always)]
unsafe fn read_atomic_avr(ptr: *const u8) -> u8 {
    let r26 = ptr.addr() as u8;
    let r27 = (ptr.addr() >> 8) as u8;
    let value: u8;
    // SAFETY: The LD instruction is atomic.
    unsafe {
        core::arch::asm!(
            "ld {value}, X",
            in("r26") r26,
            in("r27") r27,
            value = out(reg) value,
            options(nostack, preserves_flags),
        );
    }
    value
}

#[cfg(not(target_arch = "avr"))]
#[inline(always)]
unsafe fn read_atomic_generic(ptr: *const u8) -> u8 {
    let _guard = LOCK.lock();
    // SAFETY: This load is protected by the `LOCK`.
    unsafe { ptr.read() }
}

/// Low level atomic read primitive.
#[inline(always)]
unsafe fn read_atomic(ptr: *const u8) -> u8 {
    fence(SeqCst);

    // SAFETY: Our caller must pass a valid pointer.
    #[cfg(target_arch = "avr")]
    let value = unsafe { read_atomic_avr(ptr) };

    // SAFETY: Our caller must pass a valid pointer.
    #[cfg(not(target_arch = "avr"))]
    let value = unsafe { read_atomic_generic(ptr) };

    fence(SeqCst);
    value
}

#[cfg(target_arch = "avr")]
#[inline(always)]
unsafe fn write_atomic_avr(ptr: *mut u8, value: u8) {
    let r26 = ptr.addr() as u8;
    let r27 = (ptr.addr() >> 8) as u8;
    // SAFETY: The ST instruction is atomic.
    unsafe {
        core::arch::asm!(
            "st X, {value}",
            in("r26") r26,
            in("r27") r27,
            value = in(reg) value,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "avr"))]
#[inline(always)]
unsafe fn write_atomic_generic(ptr: *mut u8, value: u8) {
    let _guard = LOCK.lock();
    // SAFETY: This store is protected by the `LOCK`.
    unsafe { ptr.write(value) };
}

/// Low level atomic write primitive.
#[inline(always)]
unsafe fn write_atomic(ptr: *mut u8, value: u8) {
    fence(SeqCst);

    // SAFETY: Our caller must pass a valid pointer.
    #[cfg(target_arch = "avr")]
    unsafe {
        write_atomic_avr(ptr, value);
    }

    // SAFETY: Our caller must pass a valid pointer.
    #[cfg(not(target_arch = "avr"))]
    unsafe {
        write_atomic_generic(ptr, value);
    }

    fence(SeqCst);
}

/// Trait convert to and from the raw `u8` value.
pub trait AvrAtomicConvert: Copy {
    /// Convert from `u8` to `Self`.
    ///
    /// # Implementation hint
    ///
    /// This function must create a valid `Self` value from the `value` byte.
    /// Note that a `value` of `0_u8` must always be expected and handled correctly,
    /// because `0_u8` is the initialization value of [AvrAtomic].
    ///
    /// It is guaranteed that [AvrAtomic] only ever passes values to `from_u8`
    /// that came from `to_u8()` or are equal to `0_u8`.
    /// Note that `0_u8` can be passed to `from_u8` even if `to_u8()` never returned `0_u8`.
    fn from_u8(value: u8) -> Self;

    /// Convert from `Self` to `u8`.
    ///
    /// # Implementation hint
    ///
    /// This function must create an `u8` byte that is possible to be converted
    /// back into the same `Self` value by `from_u8`.
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
/// # Internal implementation
///
/// Note that the internal representation of the data storage always is a `u8`,
/// no matter that type `T` actually is.
#[repr(transparent)]
pub struct AvrAtomic<T> {
    // Interior mutable data.
    data: UnsafeCell<u8>,
    _phantom: PhantomData<T>,
}

impl<T> AvrAtomic<T> {
    /// Create a new [AvrAtomic] with the initial interior raw data being `0_u8`.
    #[inline(always)]
    pub const fn new() -> AvrAtomic<T> {
        Self {
            data: UnsafeCell::new(0),
            _phantom: PhantomData,
        }
    }

    /// Atomically read as raw `u8` byte.
    ///
    /// This atomic read is also a full SeqCst memory barrier.
    #[inline(always)]
    pub fn load_raw(&self) -> u8 {
        // SAFETY: The pointer passed to `read_atomic` is a valid pointer to `u8`.
        unsafe { read_atomic(self.data.get()) }
    }

    /// Atomically write as raw `u8` byte.
    ///
    /// This atomic write is also a full SeqCst memory barrier.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `value` is properly encoded to represent a valid `T`.
    #[inline(always)]
    pub unsafe fn store_raw(&self, value: u8) {
        // SAFETY: The pointer passed to `write_atomic` is a valid pointer to `u8`.
        unsafe { write_atomic(self.data.get(), value) }
    }
}

impl<T: AvrAtomicConvert> AvrAtomic<T> {
    /// Create a new [AvrAtomic] initialized to `value`.
    #[inline(always)]
    pub fn new_value(value: T) -> Self {
        let value = value.to_u8();
        Self {
            data: UnsafeCell::new(value),
            _phantom: PhantomData,
        }
    }

    /// Atomically read the current value.
    ///
    /// This atomic read is also a full SeqCst memory barrier.
    #[inline(always)]
    pub fn load(&self) -> T {
        T::from_u8(self.load_raw())
    }

    /// Atomically write a new value.
    ///
    /// This atomic write is also a full SeqCst memory barrier.
    #[inline(always)]
    pub fn store(&self, value: T) {
        // SAFETY: The `value` is properly encoded to represent `T`.
        unsafe { self.store_raw(value.to_u8()) };
    }
}

impl<T> Default for AvrAtomic<T> {
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
        assert_eq!(a.load(), 0);
        a.store(0x5A);
        assert_eq!(a.load(), 0x5A);
        a.store(0);
        assert_eq!(a.load(), 0);

        let a: AvrAtomic<u8> = AvrAtomic::new_value(99);
        assert_eq!(a.load(), 99);
    }

    #[test]
    fn test_i8() {
        let a: AvrAtomic<i8> = AvrAtomic::new();
        assert_eq!(a.load(), 0);
        a.store(-42);
        assert_eq!(a.load(), -42);
        a.store(0);
        assert_eq!(a.load(), 0);

        let a: AvrAtomic<i8> = AvrAtomic::new_value(-99);
        assert_eq!(a.load(), -99);
    }

    #[test]
    fn test_bool() {
        let a: AvrAtomic<bool> = AvrAtomic::new();
        assert!(!a.load());
        a.store(true);
        assert!(a.load());
        a.store(false);
        assert!(!a.load());

        let a: AvrAtomic<bool> = AvrAtomic::new_value(true);
        assert!(a.load());
    }
}

// vim: ts=4 sw=4 expandtab
