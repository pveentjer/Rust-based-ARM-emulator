.global _start

.text
_start:
    MOV r0, #10;

_again_outer:
    MOV r1, #10;

_again_inner:
    SUB r1, r1, #1;
    PRINTR r1;
    CBNZ r1, _again_inner;

    SUB r0, r0, #1;
    PRINTR r0;
    CBNZ r0, _again_outer;
