.section .data
    a1: .word 1
    a2: .word 1
    a3: .word 0

.section .text
.section .text
.global _start
_start:
    ADR R0, =a1
    LDR R0, [R0]
    ADR R1, =a2
    LDR R1, [R1]
_loop:
    BL _add_numbers  @ add R0 and R1 and write to R2
    PRINTR R2
    ADR R2, =a3
    STR R2, [R2]
    MOV R0, R2      @ copy R2 into R0
    B _loop
_add_numbers:
    NOP
    NOP
    NOP
    NOP
    NOP
    ADD R2, R0, R1
    BX LR