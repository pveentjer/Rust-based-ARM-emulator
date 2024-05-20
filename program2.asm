.section .data
    a: .word 10
.section .text
_start:
    ADD R0, R0, #1
    PRINTR R0
    PRINTR PC
    B _start
    EXIT