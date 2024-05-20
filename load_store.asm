.data
    a: .word 10
    b: .word 20
    c: .word 0
.text
.global _start
_start:
    ADR R0, a
    LDR R0, [R0]
    ADR R1, b
    LDR R1, [R1]
    ADD R2, R0, R1
    ADR R0, c
    STR R2, [R0]
    PRINTR R0