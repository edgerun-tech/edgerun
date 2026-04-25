/* Bare-metal linker script for edgerun unikernel */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH("x86_64")

ENTRY(_start)

SECTIONS
{
    . = 0x100000;

    .text : {
        *(.text.entry)
        *(.text)
    }

    .rodata : {
        *(.rodata)
    }

    .data : {
        *(.data)
    }

    .bss : {
        _bss_start = .;
        *(COMMON)
        *(.bss)
        _bss_end = .;
    }

    _end = .;
}