@banana
.text

foo: .word 10



banana: .word 20
.text

add r1, R2, R3

@ adr SP, foo

ldr R1, R1
ldr R1, [R1]
        @ banana


add R1, R2, #20
.data
fanothervar: .word 3

