.global _start

.data
    var_a: .dword 30
.text
_start:
    MOV r1, #10;
    MOV r2, =var_a;
    LDR r2, [r2];
    ADD r3, r1, r2;
    PRINTR r3;
