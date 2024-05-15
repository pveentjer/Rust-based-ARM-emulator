.data
    a 10
    b 0
    c 2
    d 0
.code
    LOAD [a] R0
loop:
    CALL procedure_banana
    DEC R0
    INC R1
    JNZ R0 loop
    EXIT
procedure_banana:
    PRINTR R1
    RET