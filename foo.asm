 .global start
 .section .data
    foo: .word 10
    banana: .word 20
.text
    start:
    add r2, r1, r0
    add r2, r1, r0
    add r2, r1, r0
.section .text
    foo:
    add r2, r1, r0
    add r2, r1, r0
    add r2, r1, r0
.text
    add r2, r1, r0
    banana:
    add r2, r1, r0
    add r2, r1, r0
.data
    foo: .word 10
    banana: .word 20