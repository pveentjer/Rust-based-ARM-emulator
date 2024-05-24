.global _start

.section .data
    a1: .word 1
    a2: .word 1
    loop_count: .word 10
.section .text

_add_numbers:
    ADD r2, r0, r1;
    BX lr;

_start:
    MOV r0, =a1;
    LDR r0, [r0];

    MOV r1, =a2;
    LDR r1, [r1];

    MOV r3, =loop_count;
    LDR r3, [r3];
_loop:
    BL _add_numbers;
    PRINTR r2;
    MOV r0, r2;

    PRINTR r3;
    SUB r3, r3, #1;
    CBNZ  r3, _loop;
