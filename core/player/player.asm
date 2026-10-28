; SIDbloom 6502 player. GPL-3.0-or-later. Assemble with: 64tass -b -o player.bin player.asm
;
; Mirrors core/src/engine.rs tick for tick. Entry points: init = $1000, play = $1003.
; Song data lives at DATA ($1800) in the layout produced by core/src/export/pack.rs.
; Zero page used: $FB-$FE.

DATA    = $1800

hdr_ninst = DATA+0
hdr_speed = DATA+1
hdr_filt  = DATA+2          ; SID regs $15..$18
hdr_olen  = DATA+6          ; per channel order length
hdr_olo   = DATA+9          ; per channel order list address
hdr_ohi   = DATA+12
freq_lo   = DATA+$10        ; 97 entries
freq_hi   = DATA+$80
i_ad      = DATA+$100       ; instrument arrays, 64 entries each, indexed by (inst-1)
i_sr      = DATA+$140
i_pwl     = DATA+$180
i_pwh     = DATA+$1C0
i_wlen    = DATA+$200
i_wloop   = DATA+$240       ; $FF = hold last step
i_wpl     = DATA+$280
i_wph     = DATA+$2C0
i_pspd    = DATA+$300       ; pulse sweep: signed speed, min lo/hi, max lo/hi
i_pminl   = DATA+$340
i_pminh   = DATA+$380
i_pmaxl   = DATA+$3C0
i_pmaxh   = DATA+$400
p_len     = DATA+$440       ; pattern arrays, 128 entries each
p_lo      = DATA+$4C0
p_hi      = DATA+$540
; variable-length data (wave tables, orders, patterns) follows at DATA+$600

ptr     = $FB

        * = $1000
        jmp init
        jmp play

init:
        ldy #NVARS-1
        lda #0
-       sta vars,y
        dey
        bpl -
        lda hdr_speed
        sta speed
        lda #1
        sta first
        ldx #3
-       lda hdr_filt,x
        sta filt,x
        dex
        bpl -
        rts

play:
        lda first
        beq pl_nofirst
        lda #0
        sta first
        ldx #0
-       jsr trigger_row
        inx
        cpx #3
        bne -
        jmp pl_render
pl_nofirst:
        lda tick
        bne pl_render
        ldx #0
-       jsr advance
        inx
        cpx #3
        bne -
        ldx #0
-       jsr trigger_row
        inx
        cpx #3
        bne -
pl_render:
        lda #0
        sta preempt
        lda tick
        clc
        adc #1
        cmp speed
        bne +
        inc preempt
+       ldx #0
pl_voice:
        lda #0
        sta retrig
        lda preempt
        beq pl_go
        jsr nextpos
        lda np_op
        ldy np_row
        jsr cellptr
        lda (ptr),y
        beq pl_go
        inc retrig
pl_go:  jsr render_voice
        inx
        cpx #3
        bne pl_voice
        lda fsweep
        bmi fs_neg
        clc
        adc filt+1
        bcc fs_store
        lda #$FF
        bne fs_store
fs_neg: clc
        adc filt+1
        bcs fs_store
        lda #0
fs_store:
        sta filt+1
        ldx #0
-       lda filt,x
        sta $D415,x
        inx
        cpx #4
        bne -
        inc tick
        lda tick
        cmp speed
        bcc +
        lda #0
        sta tick
+       rts

; in: X voice, A order position. out: A pattern index. Clobbers Y, ptr.
patidx:
        tay
        lda hdr_olo,x
        sta ptr
        lda hdr_ohi,x
        sta ptr+1
        lda (ptr),y
        rts

; in: X voice, A order position, Y row. out: ptr = pattern data, Y = row*4.
cellptr:
        sty cp_row
        jsr patidx
        tay
        lda p_lo,y
        sta ptr
        lda p_hi,y
        sta ptr+1
        lda cp_row
        asl
        asl
        tay
        rts

; in: X voice. out: np_op, np_row = position of the next row
nextpos:
        lda v_row,x
        clc
        adc #1
        sta np_row
        lda v_opos,x
        sta np_op
        jsr patidx
        tay
        lda np_row
        cmp p_len,y
        bcc np_done
        lda #0
        sta np_row
        inc np_op
        lda np_op
        cmp hdr_olen,x
        bcc np_done
        lda #0
        sta np_op
np_done:
        rts

advance:
        jsr nextpos
        lda np_op
        sta v_opos,x
        lda np_row
        sta v_row,x
        rts

trigger_row:
        lda v_opos,x
        ldy v_row,x
        jsr cellptr
        lda (ptr),y
        sta c_note
        iny
        lda (ptr),y
        sta c_inst
        iny
        lda (ptr),y
        sta c_fx
        iny
        lda (ptr),y
        sta c_fxp
        lda c_inst
        beq tr_noinst
        cmp hdr_ninst
        beq tr_setinst
        bcs tr_noinst
tr_setinst:
        sta v_inst,x
tr_noinst:
        lda c_note
        beq tr_fx
        cmp #$FF
        bne tr_note
        lda #0
        sta v_gate,x
        beq tr_fx
tr_note:
        lda v_inst,x
        beq tr_fx
        lda c_note
        sta v_note,x
        lda #1
        sta v_gate,x
        lda #0
        sta v_offl,x
        sta v_offh,x
        sta v_wpos,x
        sta v_vph,x
        ldy v_inst,x
        lda i_pwl-1,y
        sta v_pwl,x
        lda i_pwh-1,y
        sta v_pwh,x
        lda #1
        sta v_pwdir,x
tr_fx:
        lda c_fx
        sta v_fx,x
        lda c_fxp
        sta v_fxp,x
        lda c_fx
        cmp #$0F
        bne tr_f8
        lda c_fxp
        beq tr_end
        sta speed
        rts
tr_f8:  cmp #8
        bne tr_f9
        lda #0
        sta filt
        lda c_fxp
        sta filt+1
        rts
tr_f9:  cmp #9
        bne tr_fa
        lda c_fxp
        and #$F7
        sta filt+2
        rts
tr_fa:  cmp #$0A
        bne tr_fb
        lda c_fxp
        and #$7F
        sta filt+3
        rts
tr_fb:  cmp #$0B
        bne tr_end
        lda c_fxp
        sta fsweep
tr_end: rts

; in: f_lo/f_hi += signed d_lo/d_hi, saturating to 0..$FFFF
addclamp:
        lda f_lo
        clc
        adc d_lo
        sta f_lo
        lda f_hi
        adc d_hi
        sta f_hi
        lda d_hi
        bmi ac_neg
        bcc ac_ret
        lda #$FF
        sta f_lo
        sta f_hi
        rts
ac_neg: bcs ac_ret
        lda #0
        sta f_lo
        sta f_hi
ac_ret: rts

; in: X voice, retrig
render_voice:
        lda v_inst,x
        bne rv_go
        rts
rv_go:  tay
        lda i_wpl-1,y
        sta ptr
        lda i_wph-1,y
        sta ptr+1
        sty tmp_inst
        lda v_wpos,x
        cmp i_wlen-1,y          ; wave_pos can exceed the table after an instrument change
        bcc +
        lda i_wlen-1,y
        sec
        sbc #1
+       asl
        tay
        lda (ptr),y
        sta s_wave
        iny
        lda (ptr),y
        sta s_semis
        ldy tmp_inst
        lda v_wpos,x
        clc
        adc #1
        cmp i_wlen-1,y
        bcc rv_adv
        lda i_wloop-1,y
        cmp #$FF
        beq rv_noadv
rv_adv: sta v_wpos,x
rv_noadv:
        ; sm = wave semis (signed 16-bit)
        ldy #0
        lda s_semis
        sta sm_lo
        bpl +
        dey
+       sty sm_hi
        ; effects that act on the note index / slide accumulator
        lda v_fx,x
        bne fx_slide
        lda v_fxp,x
        beq fx_done
        lda tick
-       cmp #3
        bcc +
        sbc #3
        jmp -
+       cmp #0
        beq fx_done
        cmp #1
        bne arp2
        lda v_fxp,x
        lsr
        lsr
        lsr
        lsr
        jmp arp_add
arp2:   lda v_fxp,x
        and #$0F
arp_add:
        clc
        adc sm_lo
        sta sm_lo
        lda sm_hi
        adc #0
        sta sm_hi
        jmp fx_done
fx_slide:
        ldy tick
        beq fx_done
        cmp #1
        bne fx_down
        lda v_offl,x
        clc
        adc v_fxp,x
        sta v_offl,x
        lda v_offh,x
        adc #0
        sta v_offh,x
        jmp fx_done
fx_down:
        cmp #2
        bne fx_done
        lda v_offl,x
        sec
        sbc v_fxp,x
        sta v_offl,x
        lda v_offh,x
        sbc #0
        sta v_offh,x
fx_done:
        ; idx = clamp(note + sm, 0, 96)
        lda v_note,x
        clc
        adc sm_lo
        sta sm_lo
        lda sm_hi
        adc #0
        sta sm_hi
        bmi idx_zero
        bne idx_max
        lda sm_lo
        cmp #97
        bcc idx_set
idx_max:
        lda #96
        bne idx_set
idx_zero:
        lda #0
idx_set:
        tay
        lda freq_lo,y
        sta f_lo
        lda freq_hi,y
        sta f_hi
        lda v_offl,x
        sta d_lo
        lda v_offh,x
        sta d_hi
        jsr addclamp
        lda v_fx,x
        cmp #4
        beq rv_vib
        jmp rv_write
rv_vib:
        ; vibrato: phase += max(speed,1)*4
        lda v_fxp,x
        lsr
        lsr
        lsr
        lsr
        bne +
        lda #1
+       asl
        asl
        clc
        adc v_vph,x
        sta v_vph,x
        cmp #128
        bcs vib_hi
        sec
        sbc #64
        jmp vib_tri
vib_hi: lda #192
        sec
        sbc v_vph,x
vib_tri:
        ldy #0
        sty sign
        cmp #0
        bpl vib_abs
        eor #$FF
        clc
        adc #1
        inc sign
vib_abs:
        sta mag
        lda v_fxp,x
        and #$0F
        tay
        lda #0
        sta pr_lo
        sta pr_hi
        cpy #0
        beq vib_shift
vib_mul:
        lda pr_lo
        clc
        adc mag
        sta pr_lo
        bcc +
        inc pr_hi
+       dey
        bne vib_mul
vib_shift:
        lda pr_lo
        lsr
        lsr
        lsr
        lsr
        sta tmp
        lda pr_hi
        asl
        asl
        asl
        asl
        ora tmp
        ldy sign
        beq vib_pos
        eor #$FF
        clc
        adc #1
vib_pos:
        sta d_lo
        ldy #0
        cmp #0
        bpl +
        dey
+       sty d_hi
        jsr addclamp
rv_write:
        ; ctrl = wave | (gate && !retrig)
        lda s_wave
        ldy v_gate,x
        beq +
        ldy retrig
        bne +
        ora #1
+       sta c_ctrl
        ldy v_inst,x
        lda i_ad-1,y
        sta ad_v
        lda i_sr-1,y
        sta sr_v
        ldy vbase,x
        lda f_lo
        sta $D400,y
        lda f_hi
        sta $D401,y
        lda v_pwl,x
        sta $D402,y
        lda v_pwh,x
        sta $D403,y
        lda ad_v
        sta $D405,y
        lda sr_v
        sta $D406,y
        lda c_ctrl
        sta $D404,y
        ; ---- pulse width sweep (applies from the next tick) ----
        ldy v_inst,x
        lda i_pspd-1,y
        bne ps_go
        rts
ps_go:  sta d_lo
        ldy #0
        cmp #0
        bpl +
        dey
+       sty d_hi
        lda v_pwdir,x
        beq ps_zero
        bpl ps_add
        lda #0
        sec
        sbc d_lo
        sta d_lo
        lda #0
        sbc d_hi
        sta d_hi
        jmp ps_add
ps_zero:
        lda #0
        sta d_lo
        sta d_hi
ps_add: lda v_pwl,x
        clc
        adc d_lo
        sta f_lo
        lda v_pwh,x
        adc d_hi
        sta f_hi
        ldy v_inst,x
        lda f_hi
        bmi ps_under
        lda i_pmaxl-1,y
        cmp f_lo
        lda i_pmaxh-1,y
        sbc f_hi
        bcc ps_over
        lda f_lo
        cmp i_pminl-1,y
        lda f_hi
        sbc i_pminh-1,y
        bcc ps_under
        lda f_lo
        sta v_pwl,x
        lda f_hi
        sta v_pwh,x
        rts
ps_under:
        lda i_pminl-1,y
        sta v_pwl,x
        lda i_pminh-1,y
        sta v_pwh,x
        lda #1
        sta v_pwdir,x
        rts
ps_over:
        lda i_pmaxl-1,y
        sta v_pwl,x
        lda i_pmaxh-1,y
        sta v_pwh,x
        lda #$FF
        sta v_pwdir,x
        rts

vbase:  .byte 0, 7, 14

; ---- state (cleared by init) ----
vars:
v_opos: .fill 3
v_row:  .fill 3
v_inst: .fill 3
v_note: .fill 3
v_gate: .fill 3
v_offl: .fill 3
v_offh: .fill 3
v_wpos: .fill 3
v_fx:   .fill 3
v_fxp:  .fill 3
v_vph:  .fill 3
v_pwl:  .fill 3
v_pwh:  .fill 3
v_pwdir: .fill 3
filt:   .fill 4
fsweep: .fill 1
speed:  .fill 1
tick:   .fill 1
first:  .fill 1
NVARS = * - vars

; ---- scratch (not cleared) ----
preempt:  .fill 1
retrig:   .fill 1
np_op:    .fill 1
np_row:   .fill 1
cp_row:   .fill 1
c_note:   .fill 1
c_inst:   .fill 1
c_fx:     .fill 1
c_fxp:    .fill 1
s_wave:   .fill 1
s_semis:  .fill 1
sm_lo:    .fill 1
sm_hi:    .fill 1
f_lo:     .fill 1
f_hi:     .fill 1
d_lo:     .fill 1
d_hi:     .fill 1
tmp_inst: .fill 1
tmp:      .fill 1
sign:     .fill 1
mag:      .fill 1
pr_lo:    .fill 1
pr_hi:    .fill 1
c_ctrl:   .fill 1
ad_v:     .fill 1
sr_v:     .fill 1

        .cerror * > DATA, "player does not fit below the data area"
        .fill DATA - *, 0
