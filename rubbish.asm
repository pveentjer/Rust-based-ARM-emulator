.global _start

.section .text
_start:
    MOV R0, #1
    ADD R0, R0, #1
    PRINTR R0
    B _start