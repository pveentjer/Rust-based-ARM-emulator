.data
    a 10
    b 1
    c 2
    d 0
.code
    LOAD [a] R0
again:
    PRINTR R0
    PUSH R0
    POP R1
    PRINTR R1
    DEC R0
    JNZ R0 again
    LOAD [b] R1
    LOAD [c] R2
    ADD R1 R2 R3
    PRINTR R3