
.text
foo: .word 10
banana: .word 20
.bananana
add R1, R2, R3
adr R1, foo
ldr R1, [R1]
add R1, R2, #20
.banana
fanothervar: .word 3