.global start
.section .data
    a: .word 10
.section .text

start:
    ADD r0, r0, #1;
    PRINTR r0;
    PRINTR pc;
    B start;
    EXIT;