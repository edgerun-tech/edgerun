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
    } :text

    .rodata : {
        *(.rodata)
    } :rodata

    .data : {
        *(.data)
    } :data

    .bss : {
        *(.bss)
        *(COMMON)
    } :bss

    _bss_start = ADDR(.bss);
    _bss_end = ADDR(.bss) + SIZEOF(.bss);

    . = ALIGN(16);
    _stack_end = .;
    . += 16384;
    _stack = .;

    _end = .;
}