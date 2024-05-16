.data
    a DCD 10
.global
_start:
LDR R0, [a]
again:
PRINTR R0
DEC R0
JNZ R0, again
EXIT