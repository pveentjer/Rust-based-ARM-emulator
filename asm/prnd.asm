.global _start

.text

generate_random:
    MUL R0, R0, R1;
    ADD R0, R0, R2;
    AND R0, R0, R3;
    BX lr;

_start:
    MOV R0, =12345;
    MOV R1, =1103515245;
    MOV R2, =12345;
    MOV R3, =0x80000000;

    BL generate_random;
    PRINT R0;
    BL generate_random;
    PRINT R0;
    BL generate_random;
    PRINT R0;
