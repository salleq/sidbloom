; SIDbloom .prg starter: BASIC "10 SYS 2061", then runs the player from a CIA timer IRQ.
; GPL-3.0-or-later. Assemble: 64tass -b -o player/stub.bin player/stub.asm
; The two TIMER bytes at `timer` are patched by the exporter (PAL 19655, NTSC 17094).

        * = $0801
        .word basic_end
        .word 10
        .byte $9e
        .text "2061"
        .byte 0
basic_end:
        .word 0
start:
        .cerror * != 2061, "SYS address must match the BASIC line"
        sei
        lda #$7f
        sta $dc0d           ; no CIA interrupt sources
        lda $dc0d           ; acknowledge anything pending
        lda #<19655
timer_lo = * - 1
        sta $dc04
        lda #>19655
timer_hi = * - 1
        sta $dc05
        lda #<irq
        sta $0314
        lda #>irq
        sta $0315
        lda #0
        jsr $1000           ; player init
        lda #$81
        sta $dc0d           ; timer A interrupt on
        cli
-       jmp -

irq:
        jsr $1003           ; player play
        jmp $ea31           ; KERNAL IRQ tail (keyboard scan, acknowledge, rti)
