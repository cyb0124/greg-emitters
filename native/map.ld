SECTIONS {
    out_rx 0 : {
        /* Header: [rw_start, reloc_start, reloc_end, entry_win64] - each 4 bytes */
        LONG(__rw_start)
        LONG(__reloc_start)
        LONG(__reloc_end)
        LONG(DEFINED(entry_win64) ? entry_win64 : 0)
        KEEP(*(
            .text.entry_sysv64
            .text.entry_win64
            .text.entry_aarch64
        ))
        *(.text* .rodata* .got*)
    }
    . = ALIGN(4096);
    __rw_start = .;
    out_rw : { *(.data* .bss*) }
    __reloc_start = .;
    out_reloc : { *(.rela.dyn) }
    __reloc_end = .;
    /DISCARD/ : { *(.hash .gnu.hash .dynamic .dynstr .dynsym .comment .eh*) }
}
