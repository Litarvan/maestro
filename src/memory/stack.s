/*
 * This file implements the stack switching function.
 */

.global stack_switch_

.extern stack_switch_in

.section .text

# Performs the stack switching for the given stack and closure to execute.
stack_switch_:
	push %ebp
	mov %esp, %ebp

	mov 16(%ebp), %esp
	push 8(%ebp)
	push 12(%ebp)
	call stack_switch_in

	mov %ebp, %esp
	pop %ebp
	ret
