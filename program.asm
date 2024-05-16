.data
    a 10
    b 0
    c 2
    d 0
.global _start

_start
    LOAD [a] R0
loop:
    CALL some_procedure
    DEC R0
    INC R1
    JNZ R0 loop
    EXIT
some_procedure:
    PRINTR R1
    RET