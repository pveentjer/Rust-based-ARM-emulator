.banana
 .global start
 .section .data
    foo: .word 10
    banana: .word 20
.text
    start:
    add r2, r1, #20
    MOV r2, r1
    BL foo
.section .text
    foo:
    add Sp, SP, sp
    add r2, fp, sP
    add r2, r1, pc
.text
    add r2, r1, r0
    banana:
    add r2, r1, r0
    add r2, r1, r0
.data
    foo: .word 10
    banana: .word 20