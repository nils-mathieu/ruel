OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(x86-64)
ENTRY(_)

PHDRS
{
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)); /* Execute + Read */
    rodata  PT_LOAD    FLAGS((1 << 2));            /* Read only */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)); /* Write + Read */
}

SECTIONS
{
    . = 0xffffffff80000000;

    PROVIDE(__ruel_image_begin = .);

    .text : {
        *(.text .text.*)
    } :text

    . = ALIGN(4096);

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . = ALIGN(4096);

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    PROVIDE(__ruel_image_end = .);

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
