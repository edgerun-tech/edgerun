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
    drom_seg (R)   : ORIGIN = 0x3C000020, LENGTH = 4M
    irom_seg (RX)  : ORIGIN = 0x42000020, LENGTH = 4M
    iram_seg (RX)  : ORIGIN = 0x40380000, LENGTH = 256K
    dram_seg (RW)  : ORIGIN = 0x3FC88000, LENGTH = 256K
}

SECTIONS
{
    .iram_text : ALIGN(4) {
        KEEP(*(.text.entry))
    } > iram_seg

    .text : ALIGN(4) {
        *(.literal .literal.*)
        *(.text .text.*)
    } > iram_seg

    .appdesc : ALIGN(4) {
        KEEP(*(.flash.appdesc))
    } > drom_seg

    .data : ALIGN(4) {
        *(.rodata .rodata.*)
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

    .irom_dummy : ALIGN(4) {
        LONG(0)
    } > irom_seg

    /DISCARD/ : {
        *(.comment)
        *(.eh_frame .eh_frame.*)
        *(.eh_frame_hdr)
        *(.note .note.*)
    }
}
