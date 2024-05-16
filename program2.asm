.data
a 10
.global _start

_start
LOAD [a] R0
again:
PRINTR R0
DEC R0
JNZ R0 again
EXIT