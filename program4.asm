.data
    a DCD 10
    b DCD 0
.global
_start:
    MOV R1, #1
    MOV R2, #2
    ADD R3, R1, R2
    PRINTR R3
    STR R2, [b]
    EXIT