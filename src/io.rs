//! The I/O functions allow to communicate with the other components on the system.

use core::arch::asm;

/// Inputs a byte from the specified port.
///
/// # Safety
///
/// Reading from an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
	let ret: i8;
	asm!("in al, dx", out("al") ret, in("dx") port);

	ret as _
}

/// Inputs a word from the specified port.
///
/// # Safety
///
/// Reading from an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
	let ret: i16;
	asm!("in ax, dx", out("ax") ret, in("dx") port);

	ret as _
}

/// Inputs a long from the specified port.
///
/// # Safety
///
/// Reading from an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
	let ret: i32;
	asm!("in eax, dx", out("eax") ret, in("dx") port);

	ret as _
}

/// Outputs a byte to the specified port.
///
/// # Safety
///
/// Writing to an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
	asm!("out dx, al", in("al") value, in("dx") port);
}

/// Outputs a word to the specified port.
///
/// # Safety
///
/// Writing to an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
	asm!("out dx, ax", in("ax") value, in("dx") port);
}

/// Outputs a long to the specified port.
///
/// # Safety
///
/// Writing to an invalid port has an undefined behaviour.
/// This function is not thread safe.
#[inline(always)]
pub unsafe fn outl(port: u16, value: u32) {
	asm!("out dx, eax", in("eax") value, in("dx") port);
}
