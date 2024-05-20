.section .data
    foo1: .word 10
    bar1: .word 30
.section .text
.global _start
    _start:
    NOP
    ADD r0,r1, r2
    ADR r0, r0
    LDR r1, [r1]
    NOP
.section .data
    foo2: .word 10
    bar2: .word 30
.section .text
    _banana:
    NOP
    ADD r0,r1, r2
    ADR r0, r0
    LDR r1, [r1]
    NOP