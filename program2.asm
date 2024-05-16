.data
    a DCD 10
.global

_start:
    ADD R0, R0, #1
    PRINTR R0
    B _start
EXIT