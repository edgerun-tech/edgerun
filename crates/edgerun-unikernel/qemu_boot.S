.code16
.global _start
_start:
    cli
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %ss
    movw $0x7c00, %sp

    inb $0x92, %al
    orb $0x02, %al
    outb %al, $0x92

    lgdt gdt_desc
    movl %cr0, %eax
    orl $0x1, %eax
    movl %eax, %cr0
    ljmpl $0x08, $protected

.code32
protected:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss

    movl $0x8000, %edi
    xorl %eax, %eax
    movl $6144, %ecx
    rep stosl

    movl $0x9003, 0x8000
    movl $0xa003, 0x9000
    movl $0xb003, 0x9008
    movl $0xc003, 0x9010
    movl $0xd003, 0x9018

    movl $0x00000083, %eax
    movl $0xa000, %edi
    movl $2048, %ecx
1:
    movl %eax, (%edi)
    addl $0x200000, %eax
    addl $8, %edi
    loop 1b

    movl $0x8000, %eax
    movl %eax, %cr3
    movl %cr4, %eax
    orl $0x20, %eax
    movl %eax, %cr4

    movl $0xc0000080, %ecx
    rdmsr
    orl $0x100, %eax
    wrmsr

    movl %cr0, %eax
    orl $0x80000001, %eax
    movl %eax, %cr0
    ljmpl $0x18, $long_mode

.code64
long_mode:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movq $0x90000, %rsp
    movabsq $0x100000, %rax
    call *%rax
2:
    hlt
    jmp 2b

.align 8
gdt:
    .quad 0
    .quad 0x00cf9a000000ffff
    .quad 0x00cf92000000ffff
    .quad 0x00af9a000000ffff
gdt_desc:
    .word gdt_desc - gdt - 1
    .long gdt

.org 510
    .word 0xaa55
