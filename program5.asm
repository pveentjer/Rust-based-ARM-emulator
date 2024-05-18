.data
    a DCD 1
    b DCD 2
.global
_start:
    LDR R1, [a]
    LDR R2, [b]
    BL add_numbers
    NOP
    NOP
    NOP
    B _start
add_numbers:
    ADD R2, R0, R1
    PRINTR LR
    BX LR