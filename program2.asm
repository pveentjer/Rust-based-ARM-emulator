.data
    a: .word 10
.text
.global _start

_start:
    ADD R0, R0, #1
    PRINTR R0
    PRINTR PC
    B _start
EXIT