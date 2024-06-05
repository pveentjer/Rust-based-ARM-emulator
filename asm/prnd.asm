.global _start

.text

generate_random:
    MUL r0, r0, r1;
    ADD r0, r0, r2;
    AND r0, r0, r3;
    BX lr;

_start:
    MOV r0, #12345;
    MOV r1, #1103515245;
    MOV r2, #12345;
    MOV r3, #80000000;

    BL generate_random;
    PRINTR r0;
    BL generate_random;
    PRINTR r0;
    BL generate_random;
    PRINTR r0;
