//! The `signal` syscall allows to specify a pointer to a function to be called when a specific
//! signal is received by the current process.

use core::ffi::c_void;
use core::mem::transmute;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::regs::Regs;
use crate::process::signal::SigAction;
use crate::process::signal::SigHandler;
use crate::process::signal::Signal;
use crate::process::signal::SignalHandler;
use crate::process::signal;

/// The implementation of the `signal` syscall.
pub fn signal(regs: &Regs) -> Result<i32, Errno> {
	let signum = regs.ebx as i32;
	let handler = regs.ecx as *const c_void;

	if signum < 0 {
		return Err(errno!(EINVAL));
	}
	let signal = Signal::from_id(signum as _)?;

	let h = match handler {
		signal::SIG_IGN => SignalHandler::Ignore,
		signal::SIG_DFL => SignalHandler::Default,
		_ => {
			let handler_fn = unsafe {
				transmute::<*const c_void, SigHandler>(handler)
			};

			SignalHandler::Handler(SigAction {
				sa_handler: Some(handler_fn),
				sa_sigaction: None,
				sa_mask: 0,
				sa_flags: 0,
				sa_restorer: None,
			})
		},
	};

	let old_handler = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let old_handler = proc.get_signal_handler(&signal);
		proc.set_signal_handler(&signal, h);
		old_handler
	};

	let old_handler_ptr = match old_handler {
		SignalHandler::Ignore => signal::SIG_IGN,
		SignalHandler::Default => signal::SIG_DFL,

		SignalHandler::Handler(action) => {
			if let Some(handler) = action.sa_handler {
				let handler_ptr = unsafe {
					transmute::<SigHandler, *const c_void>(handler)
				};

				handler_ptr
			} else {
				0 as _
			}
		},
	};
	Ok(old_handler_ptr as _)
}
