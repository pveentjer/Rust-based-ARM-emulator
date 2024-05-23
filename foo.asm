.global start
.data
            foo: .word 40
.text
start:
            MOV r1, =foo;
            PRINTR r1;
again:
            PRINTR r1;
            SUB r1, r1, #1;
            PRINTR r1;
            CBNZ r1, again;
            MOV r1, #100;
            PRINTR r1;
