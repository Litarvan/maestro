Interruptions
*************

Interruptions are a feature of the CPU allowing to stop the execution of the code to handle an event.
An interruption can be Maskable or Non-Maskable. Under x86, maskable interrupts can be disabled using the ``cli`` instruction and enabled using the ``sti`` instruction.

The ``hlt`` instruction halts the CPU until an interruption happens.
The CPU usage is measured with the amount of time spent halting.

The ``int`` instruction can be used to trigger a software interruption. This is mainly used to make system calls.



Interrupt Vector
================

Under x86, the IDT (Interrupt Descriptor Table) stores the list of interrupt handlers.
The index in the table gives the id of the interrupt.
Some interrptions may push an additionnal value on the stack to give more informations.

Before returning from an interrupt, it's important to send an EOI (End Of Interrupt) command to the PIC to make sure that this interruption can be fired again.



x86 Task State Segment
----------------------

When an interruption happens while in ring 3 (TODO: place link to privilege levels), the kernel needs to go back to ring 0 to handle it.
The Task State Segment (TSS) structure indicates the segments and stack pointer to use when switching back to ring 0.



x86 Error Interrupts
--------------------

The first 32 interrupts in the vector are errors triggered by hardware.
Not catching an error interrupt shall result in a Double Fault, which must never happen.

Here is the list of error interrupts under x86:

+------+-------------------------------+-----------------+
| #    | Name                          | Additional code |
+======+===============================+=================+
| 0x00 | Divide-by-zero Error          | No              |
+------+-------------------------------+-----------------+
| 0x01 | Debug                         | No              |
+------+-------------------------------+-----------------+
| 0x02 | Non-maskable Interrupt        | No              |
+------+-------------------------------+-----------------+
| 0x03 | Breakpoint                    | No              |
+------+-------------------------------+-----------------+
| 0x04 | Overflow                      | No              |
+------+-------------------------------+-----------------+
| 0x05 | Bound Range Exceeded          | No              |
+------+-------------------------------+-----------------+
| 0x06 | Invalid Opcode                | No              |
+------+-------------------------------+-----------------+
| 0x07 | Device Not Available          | No              |
+------+-------------------------------+-----------------+
| 0x08 | Double Fault                  | Yes             |
+------+-------------------------------+-----------------+
| 0x09 | Coprocessor Segment Overrun   | No              |
+------+-------------------------------+-----------------+
| 0x0a | Invalid TSS                   | Yes             |
+------+-------------------------------+-----------------+
| 0x0b | Segment Not Present           | Yes             |
+------+-------------------------------+-----------------+
| 0x0c | Stack-Segment Fault           | Yes             |
+------+-------------------------------+-----------------+
| 0x0d | General Protection Fault      | Yes             |
+------+-------------------------------+-----------------+
| 0x0e | Page Fault                    | Yes             |
+------+-------------------------------+-----------------+
| 0x0f | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x10 | x87 Floating-Point Exception  | No              |
+------+-------------------------------+-----------------+
| 0x11 | Alignment Check               | Yes             |
+------+-------------------------------+-----------------+
| 0x12 | Machine Check                 | No              |
+------+-------------------------------+-----------------+
| 0x13 | SIMD Floating-Point Exception | No              |
+------+-------------------------------+-----------------+
| 0x14 | Virtualization Exception      | No              |
+------+-------------------------------+-----------------+
| 0x15 | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x16 | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x17 | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x18 | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x19 | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x1a | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x1b | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x1c | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x1d | Reserved                      | No              |
+------+-------------------------------+-----------------+
| 0x1e | Security Exception            | Yes             |
+------+-------------------------------+-----------------+
| 0x1f | Reserved                      | No              |
+------+-------------------------------+-----------------+



x86 Triple Fault
----------------

A Triple Fault is a special kind of error which is triggered when a Double Fault is not caught.
The effect is to reset the CPU and perform the boot process again, which must never happen.



Mutex considerations
--------------------

When an interruption is being handled, the currently running code is paused until the interrupt handler returns.
However, if a mutex to a sensitive resource is locked, then an interrupt is received, if this interrupt also tries to use the same resource, the kernel will be deadlocked since the mutex cannot be unlocked until the interrupt returns.

To counteract this problem, mutexes implement a feature allowing to disable interrupts for the time during which the sensitive resource is accessed. The interrupt state is then restored when the mutex is unlocked.



x86 System calls
================

A system call allows processes to communicate with the kernel. It can be triggered by using the interrupt ``0x80`` under x86:

.. code:: asm

    int $0x80

The id of the syscall is stored in the register ``%eax``.  Other registers are used to pass arguments with the syscall.
Each of the following registers are used to pass arguments to the system call, in this order:

- ``%ebx``
- ``%ecx``
- ``%edx``
- ``%esi``
- ``%edi``
- ``%ebp``
