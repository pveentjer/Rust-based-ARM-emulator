.data
    a DCD 10
    b DCD 0
    c DCD 2
    d DCD 0
.global
_start:
    LDR R0, [a]
loop:
    CALL some_procedure
    DEC R0
    INC R1
    JNZ R0, loop
    EXIT
some_procedure:
    PRINTR R1
    RET