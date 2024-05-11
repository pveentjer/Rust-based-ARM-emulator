.data
    a 10
    b 20
    c 10
    d 0
.code
    LOAD [a] R1
    INC R1
    INC R1
    INC R1
    LOAD [b] R2
    ADD R1 R2 R3
    PRINTR R3
    STORE R4 [d]