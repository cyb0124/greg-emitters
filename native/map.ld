SECTIONS {
    out_rx 0 : {
        KEEP(*(
            .text.entry_sysv64
            .text.entry_win64
            .text.entry_aarch64
        ))
        *(.text* .rodata* .got*)
    }
    . = ALIGN(4096);
    out_rw : { *(.data* .bss*) }
    out_reloc : { *(.rela.dyn) }
    /DISCARD/ : { *(.hash .gnu.hash .dynamic .dynstr .dynsym .comment .eh*) }
}
