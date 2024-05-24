.global _start

.data
    var_a: .word 10
    var_b: .word 20
    var_c: .word 0

.text

_start:
    MOV r0, =var_a;
    LDR r0, [r0];
    MOV r1, =var_b;
    LDR r1, [r1];
    ADD r2, r0, r1;
    MOV r0, =var_c;
    STR r2, [r0];
    PRINTR r0;