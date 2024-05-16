.data
    a 10
    b 10
    c 0
.global
_start:
    LDR R1, [a]
    LDR R2, [b]
    ADD R3, R1, R2
    STR R3, [c]