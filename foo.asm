.global start
 .section .data
 .section .data
    foo: .word 10
    banana: .word 20
.text
    start:
    add r5, r1, #20;
    MOV r2, r1;
    NEG r1, r4;
    BL foo;
    BL start;
.section .text
    NOP ;
    banana:
    foo:
    add sp, sp, sp;
    add r5, fp, sp;
    add r2, r1, pc;
.text
    add r30, r1, r0;
    add sp, r0, r0;
    add fp, r1, r0;
.data
    foo1: .word 10
    bananadddd: .word 20
