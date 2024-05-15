; dooo

            ; fooo ss;ssk

.data
    a 10
.code
    LOAD [a] R0
again:
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP

; doo

    NOP
    NOP
    NOP
    NOP

    NOP
    NOP
    NOP
    NOP ; comments
    NOP
    NOP
    NOP
    NOP
    NOP
PRINTR R0
DEC R0
JNZ R0 again
EXIT