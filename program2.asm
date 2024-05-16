.data
    a DCD 10
.global
_start:
LDR R0, [a]
again:
PRINTR R0
SUB R0,R0,1
JNZ R0, again
EXIT