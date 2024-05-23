.global start
.text
    MOV r1, #10;
    start:
    PRINTR r1;
    SUB r1, r1, #1;
    CBZ r1, start;
