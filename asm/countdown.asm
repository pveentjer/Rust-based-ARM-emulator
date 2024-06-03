.global _start

.text
_start:
    MOV r1, #10;
_again:
    SUB r1, r1, #1;
    PRINTR r1;
    CBNZ r1, _again;
    MOV r1, #10000;
    PRINTR r1;