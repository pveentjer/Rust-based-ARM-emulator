.data ; banana
a 10    ; fop
b 20
c 10
.code ; bar
foo:
LOAD [A] R4 ; bar
LOAD [B] R7
INC R1
DEC R8