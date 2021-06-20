//! This file handles interruptions, it provides an interface allowing to register callbacks for
//! each interrupts. Each callback has a priority number and is called in descreasing order.

use core::mem::MaybeUninit;
use crate::errno::Errno;
use crate::idt::pic;
use crate::idt;
use crate::process::tss;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::*;
use crate::util;

/// The list of interrupt error messages ordered by index of the corresponding interrupt vector.
#[cfg(any(config_general_arch = "x86", config_general_arch = "x86_64"))]
static ERROR_MESSAGES: &[&str] = &[
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignement Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown"
];

/// Returns the error message corresponding to the given interrupt vector index `i`.
fn get_error_message(i: u32) -> &'static str {
	if (i as usize) < ERROR_MESSAGES.len() {
		ERROR_MESSAGES[i as usize]
	} else {
		"Unknown"
	}
}

/// The action to execute after the interrupt handler has returned.
pub enum InterruptResultAction {
	/// Resumes execution of the code where it was interrupted.
	Resume,
	/// Goes back to the kernel loop, waiting for another interruption.
	Loop,
	/// Makes the kernel panic.
	Panic,
}

/// Enumeration telling which action will be executed after an interrupt handler.
pub struct InterruptResult {
	/// Tells whether to skip execution of the next interrupt handlers (with lower priority).
	skip_next: bool,
	/// The action to execute after the handler. The last handler decides which action to execute
	/// unless the `skip_next` variable is set to `true`.
	action: InterruptResultAction,
}

impl InterruptResult {
	/// Creates a new instance.
	pub fn new(skip_next: bool, action: InterruptResultAction) -> Self {
		Self {
			skip_next,
			action,
		}
	}
}

/// Trait representing a callback that aims to be called whenever an associated interruption is
/// triggered.
pub trait Callback {
	/// Calls the callback.
	/// `id` is the id of the interrupt.
	/// `code` is an optional code associated with the interrupt. If no code is given, the value is
	/// `0`.
	/// `regs` the values of the registers when the interruption was triggered.
	/// `ring` tells the ring at which the code was running.
	/// If the function returns `false`, the kernel shall panic.
	fn call(&mut self, id: u32, code: u32, regs: &util::Regs, ring: u32) -> InterruptResult;
}

/// Structure wrapping a callback to insert it into a linked list.
struct CallbackWrapper {
	/// The priority associated with the callback. Higher value means higher priority
	priority: u32,
	/// The callback
	callback: Box<dyn Callback>,
}

/// Structure used to detect whenever the object owning the callback is destroyed, allowing to
/// unregister it automatically.
pub struct CallbackHook {
	// TODO Store informations on the callback
}

impl CallbackHook {
	/// Creates a new instance.
	fn new() -> Self {
		Self {
			// TODO
		}
	}
}

impl Drop for CallbackHook {
	fn drop(&mut self) {
		// TODO Remove the callback
	}
}

/// List containing vectors that store callbacks for every interrupt watchdogs.
static mut CALLBACKS: MaybeUninit<[Mutex<Vec<CallbackWrapper>>; idt::ENTRIES_COUNT as _]>
	= MaybeUninit::uninit();

/// Initializes the events handler.
/// This function must be called only once when booting.
pub fn init() {
	let callbacks = unsafe { // Safe because called only once
		CALLBACKS.assume_init_mut()
	};

	for c in callbacks {
		*c.lock().get_mut() = Vec::new();
	}
}

/// Registers the given callback and returns a reference to it.
/// `id` is the id of the interrupt to watch.
/// `priority` is the priority for the callback. Higher value means higher priority.
/// `callback` is the callback to register.
///
/// If the `id` is invalid or if an allocation fails, the function shall return an error.
pub fn register_callback<T: 'static + Callback>(id: usize, priority: u32, callback: T)
	-> Result<CallbackHook, Errno> {
	debug_assert!(id < idt::ENTRIES_COUNT);

	idt::wrap_disable_interrupts(|| {
		let mut guard = unsafe {
			CALLBACKS.assume_init_mut()
		}[id].lock();
		let vec = &mut guard.get_mut();

		let index = {
			let r = vec.binary_search_by(| x | {
				x.priority.cmp(&priority)
			});

			if let Err(l) = r {
				l
			} else {
				r.unwrap()
			}
		};

		vec.insert(index, CallbackWrapper {
			priority,
			callback: Box::new(callback)?,
		})?;

		Ok(CallbackHook::new()) // TODO
	})
}

/// This function is called whenever an interruption is triggered.
/// `id` is the identifier of the interrupt type. This value is architecture-dependent.
/// `code` is an optional code associated with the interrupt. If the interrupt type doesn't have a
/// code, the value is `0`.
/// `regs` is the state of the registers at the moment of the interrupt.
/// `ring` tells the ring at which the code was running.
#[no_mangle]
pub extern "C" fn event_handler(id: u32, code: u32, ring: u32, regs: &util::Regs) {
	let action = {
		let callbacks = unsafe {
			CALLBACKS.assume_init_mut()[id as usize].get_mut_payload()
		};

		let mut last_action = {
			if (id as usize) < ERROR_MESSAGES.len() {
				InterruptResultAction::Panic
			} else {
				InterruptResultAction::Resume
			}
		};

		for i in 0..callbacks.len() {
			let result = (callbacks[i].callback).call(id, code, regs, ring);
			last_action = result.action;
			if result.skip_next {
				break;
			}
		}

		last_action
	};

	match action {
		InterruptResultAction::Resume => {},
		InterruptResultAction::Loop => {
			pic::end_of_interrupt(id as _);
			// TODO Fix: Use of loop action before TSS init shall result in undefined behaviour
			// TODO Fix: The stack might be removed while being used (example: process is
			// killed, its exit status is retrieved from another CPU core and then the process
			// is removed)
			unsafe {
				crate::loop_reset(tss::get().esp0 as _);
			}
		},
		InterruptResultAction::Panic => {
			crate::kernel_panic!(get_error_message(id), code);
		},
	}
}
