//! The virtual memory makes the kernel able to isolate processes, which is essential for modern
//! systems.

// TODO Make this file fully cross-platform

#[cfg(any(config_general_arch = "x86", config_general_arch = "x86_64"))]
pub mod x86;

use core::ffi::c_void;
use crate::errno::Errno;
use crate::util::FailableClone;
use crate::util::boxed::Box;

/// Trait representing virtual memory context handler. This trait is the interface to manipulate
/// virtual memory on any architecture. Each architecture has its own structure implementing this
/// trait.
pub trait VMem: FailableClone {
	/// Translates the given virtual address `ptr` to the corresponding physical address. If the
	/// address is not mapped, the function returns None.
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void>;

	/// Tells whether the given pointer `ptr` is mapped or not.
	fn is_mapped(&self, ptr: *const c_void) -> bool {
		self.translate(ptr) != None
	}

	/// Maps the the given physical address `physaddr` to the given virtual address `virtaddr` with
	/// the given flags.
	fn map(&mut self, physaddr: *const c_void, virtaddr: *const c_void, flags: u32)
		-> Result<(), Errno>;
	/// Maps the given range of physical address `physaddr` to the given range of virtual address
	/// `virtaddr`. The range is `pages` pages large.
	fn map_range(&mut self, physaddr: *const c_void, virtaddr: *const c_void, pages: usize,
		flags: u32) -> Result<(), Errno>;

	/// Maps the physical address `ptr` to the same address in virtual memory with the given flags
	/// `flags`.
	fn identity(&mut self, ptr: *const c_void, flags: u32) -> Result<(), Errno> {
		self.map(ptr, ptr, flags)
	}
	/// Identity maps a range beginning at physical address `from` with pages `pages` and flags
	/// `flags`.
	fn identity_range(&mut self, ptr: *const c_void, pages: usize, flags: u32)
		-> Result<(), Errno> {
		self.map_range(ptr, ptr, pages, flags)
	}

	/// Unmaps the page at virtual address `virtaddr`.
	fn unmap(&mut self, virtaddr: *const c_void) -> Result<(), Errno>;
	/// Unmaps the given range beginning at virtual address `virtaddr` with size of `pages` pages.
	fn unmap_range(&mut self, virtaddr: *const c_void, pages: usize) -> Result<(), Errno>;

	/// Binds the virtual memory context handler.
	fn bind(&self);
	/// Tells whether the handler is bound or not.
	fn is_bound(&self) -> bool;
	/// Flushes the modifications of the context if bound. This function should be called after
	/// applying modifications to the context.
	fn flush(&self);
}

/// Creates a new virtual memory context handler for the current architecture.
pub fn new() -> Result::<Box::<dyn VMem>, Errno> {
	Ok(Box::new(x86::X86VMem::new()?)? as Box::<dyn VMem>)
}

/// Clones the virtual memory context handler `vmem`.
pub fn clone(vmem: &Box::<dyn VMem>) -> Result::<Box::<dyn VMem>, Errno> {
	let vmem = unsafe {
		&*(vmem.as_ptr() as *const x86::X86VMem)
	};
	Ok(Box::new(vmem.failable_clone()?)? as Box::<dyn VMem>)
}

/// Creates and loads the kernel's virtual memory context handler, protecting its code from
/// writing.
pub fn kernel() -> Result<Box::<dyn VMem>, Errno> {
	let kernel_vmem = new()?;
	kernel_vmem.bind();
	Ok(kernel_vmem)
}

/// Tells whether the read-only pages protection is enabled.
pub fn is_write_lock() -> bool {
	unsafe {
		(x86::cr0_get() & (1 << 16)) != 0
	}
}

/// Sets whether the kernel can write to read-only pages.
pub unsafe fn set_write_lock(lock: bool) {
	if lock {
		x86::cr0_set(1 << 16);
	} else {
		x86::cr0_clear(1 << 16);
	}
}

/// Executes the closure given as parameter. During execution, the kernel can write on read-only
/// pages. The state of the write lock is restored after the closure's execution.
pub unsafe fn write_lock_wrap<F: Fn() -> T, T>(f: F) -> T {
	let lock = is_write_lock();
	set_write_lock(false);
	let result = f();
	set_write_lock(lock);

	result
}

/// Executes the given closure `f` while being bound to the given virtual memory context `vmem`.
/// After execution, the function restores the previous context.
pub fn vmem_switch<F: FnMut() -> T, T>(vmem: &dyn VMem, mut f: F) -> T {
	if vmem.is_bound() {
		f()
	} else {
		let cr3 = unsafe {
			x86::cr3_get()
		};
		vmem.bind();

		let result = f();

		unsafe {
			x86::paging_enable(cr3 as _);
		}

		result
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::memory;

	#[test_case]
	fn vmem_basic0() {
		let vmem = new().unwrap();
		for i in ((0 as usize)..(0xc0000000 as usize)).step_by(memory::PAGE_SIZE) {
			assert_eq!(vmem.translate(i as _), None);
		}
	}

	#[test_case]
	fn vmem_basic1() {
		let vmem = new().unwrap();
		for i in (0..0x40000000).step_by(memory::PAGE_SIZE) {
			let virt_ptr = ((memory::PROCESS_END as usize) + i) as _;
			let result = vmem.translate(virt_ptr);
			assert_ne!(result, None);
			let phys_ptr = result.unwrap();
			assert_eq!(phys_ptr, i as _);
		}
	}

	#[test_case]
	fn vmem_map0() {
		let mut vmem = new().unwrap();
		vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();

		for i in ((0 as usize)..(0xc0000000 as usize)).step_by(memory::PAGE_SIZE) {
			if i >= 0x100000 && i < 0x101000 {
				let result = vmem.translate(i as _);
				assert!(result.is_some());
				assert_eq!(result.unwrap(), i as _);
			} else {
				assert_eq!(vmem.translate(i as _), None);
			}
		}
	}

	#[test_case]
	fn vmem_map1() {
		let mut vmem = new().unwrap();
		vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();
		vmem.map(0x200000 as _, 0x100000 as _, 0).unwrap();

		for i in ((0 as usize)..(0xc0000000 as usize)).step_by(memory::PAGE_SIZE) {
			if i >= 0x100000 && i < 0x101000 {
				let result = vmem.translate(i as _);
				assert!(result.is_some());
				assert_eq!(result.unwrap(), (0x100000 + i) as _);
			} else {
				assert_eq!(vmem.translate(i as _), None);
			}
		}
	}

	// TODO More tests on map
	// TODO Test on map_range

	#[test_case]
	fn vmem_unmap0() {
		let mut vmem = new().unwrap();
		vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();
		vmem.unmap(0x100000 as _).unwrap();

		for i in ((0 as usize)..(0xc0000000 as usize)).step_by(memory::PAGE_SIZE) {
			assert_eq!(vmem.translate(i as _), None);
		}
	}

	// TODO More tests on unmap
	// TODO Test on unmap_range

	// TODO Add more tests
}
