/* Minimal ESP32-S3 Xtensa image layout for early Edgerun bring-up.
 *
 * This is deliberately small: it gives the Rust binary a no_std link target
 * without pulling ESP-IDF's full project build into the unikernel crate.
 * Flash/ROM boot headers still need a real ESP32-S3 image packer before this
 * can boot directly from reset.
 */
OUTPUT_ARCH("xtensa")
ENTRY(_start)

MEMORY
{
    iram_seg (RX)  : ORIGIN = 0x40380000, LENGTH = 256K
    dram_seg (RW)  : ORIGIN = 0x3FC88000, LENGTH = 256K
}

SECTIONS
{
    .text : ALIGN(4) {
        KEEP(*(.text.entry))
        *(.literal .literal.*)
        *(.text .text.*)
    } > iram_seg

    .rodata : ALIGN(4) {
        *(.rodata .rodata.*)
    } > dram_seg

    .data : ALIGN(4) {
        *(.data .data.*)
    } > dram_seg

    .bss (NOLOAD) : ALIGN(4) {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    } > dram_seg

    . = ALIGN(16);
    _stack_end = .;
    . += 16384;
    _stack = .;

    _end = .;

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame .eh_frame.*)
        *(.eh_frame_hdr)
        *(.note .note.*)
    }
}
