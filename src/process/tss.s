.global tss_gdt_entry
.global tss_flush

tss_gdt_entry:
	movl $gdt_tss, %eax

	ret

tss_flush:
	movw $TSS_OFFSET, %ax
	ltr %ax

	ret
