.data
    a DCD 10
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