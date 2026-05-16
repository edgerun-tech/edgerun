/* Bare-metal linker script for edgerun unikernel */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH("x86_64")

ENTRY(_start)

SECTIONS
{
    . = 0x100000;

    .text : {
        KEEP(*(.text.entry))
        KEEP(*(.multiboot))
        *(.text .text.*)
    } :text

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    .data : {
        *(.data .data.*)
    } :data

    . = ALIGN(16);
    _bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :bss
    _bss_end = .;

    . = ALIGN(16);
    _stack_end = .;
    . += 131072;
    _stack = .;

    _end = .;

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame .eh_frame.*)
        *(.eh_frame_hdr)
        *(.note .note.*)
    }
}
