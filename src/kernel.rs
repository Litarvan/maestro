//! Maestro is a Unix kernel written in Rust. This reference documents interfaces for modules and
//! the kernel's internals.

#![no_std]

#![allow(unused_attributes)]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(coerce_unsized)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_mut_refs)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]
#![feature(slice_ptr_len)]
#![feature(stmt_expr_attributes)]
#![feature(trait_upcasting)]
#![feature(unsize)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]
#![allow(incomplete_features)]

#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

pub mod acpi;
pub mod cmdline;
pub mod cpu;
pub mod crypto;
pub mod debug;
pub mod device;
pub mod elf;
#[macro_use]
pub mod errno;
pub mod event;
pub mod file;
pub mod gdt;
#[macro_use]
pub mod idt;
pub mod io;
pub mod limits;
pub mod logger;
pub mod memory;
pub mod module;
pub mod multiboot;
#[macro_use]
pub mod panic;
pub mod pit;
#[macro_use]
pub mod print;
pub mod process;
pub mod selftest;
pub mod syscall;
pub mod time;
pub mod tty;
pub mod types;
#[macro_use]
pub mod util;
#[macro_use]
pub mod vga;

use core::ffi::c_void;
use core::panic::PanicInfo;
use core::ptr::null;
use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::process::Process;
use crate::process::exec::ExecInfo;
use crate::process::exec;
use crate::util::boxed::Box;
use crate::util::lock::Mutex;

/// The kernel's name.
pub const NAME: &str = "maestro";
/// Current kernel version.
pub const VERSION: &str = "1.0";

/// The path to the init process binary.
const INIT_PATH: &[u8] = b"/sbin/init";

extern "C" {
	fn kernel_wait();
	fn kernel_loop() -> !;
	fn kernel_loop_reset(stack: *mut c_void) -> !;
	fn kernel_halt() -> !;
}

/// Makes the kernel wait for an interrupt, then returns.
/// This function enables interrupts.
pub fn wait() {
	unsafe {
		kernel_wait();
	}
}

/// Enters the kernel loop and processes every interrupts indefinitely.
pub fn enter_loop() -> ! {
	unsafe {
		kernel_loop();
	}
}

/// Resets the stack to the given value, then calls `enter_loop`.
/// The function is unsafe because the pointer passed in parameter might be invalid.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	kernel_loop_reset(stack);
}

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	unsafe {
		kernel_halt();
	}
}

/// Field storing the kernel's virtual memory context.
static KERNEL_VMEM: Mutex<Option<Box<dyn VMem>>> = Mutex::new(None);

/// Initializes the kernel's virtual memory context.
fn init_vmem() -> Result<(), Errno> {
	let mut kernel_vmem = vmem::new()?;

	// TODO If Meltdown mitigation is enabled, only allow read access to a stub of the
	// kernel for interrupts

	// TODO Enable GLOBAL in cr4

	// Mapping the kernelspace
	kernel_vmem.map_range(null::<c_void>(),
		memory::PROCESS_END,
		memory::get_kernelspace_size() / memory::PAGE_SIZE,
		vmem::x86::FLAG_WRITE | vmem::x86::FLAG_GLOBAL)?;

	// Mapping VGA's buffer
	let vga_flags = vmem::x86::FLAG_CACHE_DISABLE | vmem::x86::FLAG_WRITE_THROUGH
		| vmem::x86::FLAG_WRITE;
	kernel_vmem.map_range(vga::BUFFER_PHYS as _, vga::get_buffer_virt() as _, 1, vga_flags)?;

	// Making the kernel image read-only
	kernel_vmem.protect_kernel()?;

	// Assigning to the global variable
	*KERNEL_VMEM.lock().get_mut() = Some(kernel_vmem);

	// Binding the kernel virtual memory context
	bind_vmem();
	Ok(())
}

/// Returns the kernel's virtual memory context.
pub fn get_vmem() -> &'static Mutex<Option<Box<dyn VMem>>> {
	&KERNEL_VMEM
}

/// Tells whether memory management has been fully initialized.
pub fn is_memory_init() -> bool {
	get_vmem().lock().get().is_some()
}

/// Binds the kernel's virtual memory context.
/// If the kernel vmem is not initialized, the function does nothing.
pub fn bind_vmem() {
	let guard = KERNEL_VMEM.lock();

	if let Some(vmem) = guard.get().as_ref() {
		vmem.bind();
	}
}

extern "C" {
	fn test_process();
}

/// Launches the init process.
/// `init_path` is the path to the init program.
fn init(init_path: &[u8]) -> Result<(), Errno> {
	let mutex = Process::new()?;
	let mut lock = mutex.lock();
	let proc = lock.get_mut();

	if cfg!(config_debug_testprocess) {
		// The pointer to the beginning of the test process
		let test_begin = unsafe {
			core::mem::transmute::<unsafe extern "C" fn(), *const c_void>(test_process)
		};

		proc.init_dummy(test_begin)
	} else {
		let path = Path::from_str(INIT_PATH, false)?;

		// The initial environment
		let mut env = vec![
			&b"PATH=/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin"[..],
			&b"TERM=maestro"[..],
		]?;
		if cfg!(config_debug_rust_backtrace) {
			env.push(&b"RUST_BACKTRACE=full"[..])?;
		}

		let file = {
			let fcache_mutex = fcache::get();
			let mut fcache_guard = fcache_mutex.lock();
			let fcache = fcache_guard.get_mut().as_mut().unwrap();

			fcache.get_file_from_path(&path, 0, 0, true)?
		};
		let mut file_guard = file.lock();

		let exec_info = ExecInfo {
			uid: proc.get_uid(),
			euid: proc.get_euid(),
			gid: proc.get_gid(),
			egid: proc.get_egid(),

			argv: &vec![
				init_path
			]?,
			envp: &env,
		};
		let program_image = exec::build_image(file_guard.get_mut(), exec_info)?;

		exec::exec(proc, program_image)
	}
}

/// This is the main function of the Rust source code, responsible for the initialization of the
/// kernel. When calling this function, the CPU must be in Protected Mode with the GDT loaded with
/// space for the Task State Segment.
/// `magic` is the magic number passed by Multiboot.
/// `multiboot_ptr` is the pointer to the Multiboot booting informations structure.
#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	cli!();
	// Initializing TTY
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		kernel_panic!("Bootloader non compliant with Multiboot2!");
	}

	// Initializing IDT, PIT and events handler
	idt::init();
	pit::init();
	event::init();

	// Ensuring the CPU has SSE
	if !cpu::sse::is_present() {
		kernel_panic!("SSE support is required to run this kernel :(");
	}
	cpu::sse::enable();

	// Reading multiboot informations
	multiboot::read_tags(multiboot_ptr);

	// Initializing memory allocation
	memory::memmap::init(multiboot_ptr);
	if cfg!(config_debug_debug) {
		memory::memmap::print_entries();
	}
	memory::alloc::init();
	memory::malloc::init();

	if init_vmem().is_err() {
		kernel_panic!("Cannot initialize kernel virtual memory!");
	}

	// From here, the kernel considers that memory management has been fully initialized

	// Performing kernel self-tests
	#[cfg(test)]
	#[cfg(config_debug_test)]
	kernel_selftest();

	// Parsing bootloader command line arguments
	let cmdline = multiboot::get_boot_info().cmdline.unwrap_or(b"");
	let args_parser = cmdline::ArgsParser::parse(&cmdline);
	if let Err(e) = args_parser {
		e.print();
		halt();
	}
	let args_parser = args_parser.unwrap();
	logger::init(args_parser.is_silent());

	println!("Booting Maestro kernel version {}", VERSION);

	println!("Initializing ACPI...");
	acpi::init();

	println!("Initializing ramdisks...");
	device::storage::ramdisk::create()
		.unwrap_or_else(| e | kernel_panic!("Failed to create ramdisks! ({})", e));
	println!("Initializing devices management...");
	device::init()
		.unwrap_or_else(| e | kernel_panic!("Failed to initialize devices management! ({})", e));

	let (root_major, root_minor) = args_parser.get_root_dev();
	println!("Root device is {} {}", root_major, root_minor);
	println!("Initializing files management...");
	file::init(device::DeviceType::Block, root_major, root_minor)
		.unwrap_or_else(| e | kernel_panic!("Failed to initialize files management! ({})", e));
	device::default::create()
		.unwrap_or_else(| e | kernel_panic!("Failed to create default devices! ({})", e));

	println!("Initializing processes...");
	process::init().unwrap_or_else(| e | kernel_panic!("Failed to init processes! ({})", e));

	let init_path = args_parser.get_init_path().as_ref()
		.map(| s | s.as_bytes())
		.unwrap_or(INIT_PATH);
	init(init_path).unwrap_or_else(| e | kernel_panic!("Cannot execute init process: {}", e));
	enter_loop();
}

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	#[cfg(test)]
	if selftest::is_running() {
		println!("FAILED\n");
		println!("Error: {}\n", panic_info);

		#[cfg(config_debug_qemu)]
		selftest::qemu::exit(selftest::qemu::FAILURE);
		#[cfg(not(config_debug_qemu))]
		halt();
	}

	if let Some(s) = panic_info.message() {
		panic::rust_panic(s);
	} else {
		crate::kernel_panic!("Rust panic (no payload)");
	}
}

/// Function that is required to be implemented by the Rust compiler and is used only when
/// panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
