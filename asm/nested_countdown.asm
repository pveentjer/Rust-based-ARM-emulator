.global _start

.text
_start:
    MOV r0, #3;

_again_outer:
    MOV r1, #3;

_again_inner:
    SUB r1, r1, #1;
    PRINTR r1;
    ADD r4, r4, #1;
    PRINTR r4;
    CBNZ r1, _again_inner;

    SUB r0, r0, #1;
    PRINTR r0;
    CBNZ r0, _again_outer;
