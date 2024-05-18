.data
    a: .word 1
    b: .word 1
    c: .word 0
.text
.global _start
_start:
    LDR R0, [a]
    LDR R1, [b]
_loop:
    BL add_numbers  @ add R0 and R1 and write to R2
    PRINTR R2
    STR R2, [c]
    MOV R0, R2      @ copy R2 into R0
    B _loop
add_numbers:
    NOP
    NOP
    NOP
    NOP
    NOP
    ADD R2, R0, R1
    BX LR