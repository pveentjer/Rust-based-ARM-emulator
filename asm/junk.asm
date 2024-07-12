.data
    var_b: .dword 20
    var_a: .dword 20
.text
    MOV r0, =var_a;
    LDR r1, [r0];
    ADD r1, r1, #10;
    PRINTR r1;
    MOV r0, =var_a;
    STR r1, [r0];
