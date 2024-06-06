.global start
.data
            var_a: .dword 40
            var_b: .dword 0
.text
start:
            MOV r1, =var_a;
            LDR r1, [r1];
            PRINTR r1;
again:
            PRINTR r1;
            SUB r1, r1, #1;
            PRINTR r1;
            MOV r2, r1;
            MOV r3, =var_b;
            STR r2, [r3];
            CBNZ r1, again;
            MOV r1, #100;
            PRINTR r1;
