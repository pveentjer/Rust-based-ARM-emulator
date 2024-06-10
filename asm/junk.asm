.data
    var_a: .dword 0
.text
    MOV r0, #1;
    MOV r1, =var_a;
    MOV r2, #0;
loop:
    ADD r1, r1, #1;
    SUB r0, r0, #1;
    STR r2, [r1];
    CBNZ r0, loop;