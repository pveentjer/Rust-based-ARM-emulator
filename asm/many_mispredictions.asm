.global _start

.section .text

_print_one:
    MOV r3, #1;
    PRINTR r3;
    BX lr;

_print_zero:
    MOV r3, #0;
    PRINTR r3;
    BX lr;

_start:
    MOV r0, #10;
    _loop:
    SUB r0, r0, #1;
    MOV r1, r1, #2;
    CBZ r1, _print_zero;
    CBNZ r1, _print_one;
    CBNZ r0, _loop;
