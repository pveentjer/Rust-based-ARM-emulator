.section .data
    a1: .word 1
    a2: .word 1
    a3: .word 0
.section .text
.global _start
_start:
    MOV r0, =a1;
    LDR r0, [r0];
    MOV r1, =a2;
    LDR r1, [r1];
_loop:
    BL _add_numbers;
    PRINTR r2;
    MOV r2, =a3;
    STR r2, [r2];
    MOV r0, r2;
    B _loop;
_add_numbers:
    ADD r2, r0, r1;
    BX lr;