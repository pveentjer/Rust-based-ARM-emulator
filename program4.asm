.data
    a DCD 10
    b DCD 10
    c DCD 0
.global
_start:
    LDR R1, [a]
    LDR R2, [b]
    ADD R3, R1, R2
    STR R3, [c]
    EXIT