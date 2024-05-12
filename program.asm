.data
    a 10
    b 20
    c 10
    d 0
.code
    LOAD [a] R0
    LOAD [b] R1
foo:
    PRINTR R0
    DEC R0
    JNZ R0 foo
    PRINTR R1