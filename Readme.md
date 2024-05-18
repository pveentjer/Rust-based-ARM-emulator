# Rust based ARM CPU emulator

## About The Project
The primary aim of this project is to provide a possible implementation of modern
processors in Software (Rust) for the ARM instruction set.

The goal of the project is to provide some insights in how modern processors potentially
could work.

The goal of the project is not to provide a super fast implementation; for that
code generation using binary translation would be orders of magnitude faster.  

## Warning

This project is toy project. I want to upgrade my Rust skills and I needed a serious
enough challenge to work that can start out single threaded, and eventually could become
multithreaded. This way I can increase Rust specific complexity in a controlled rate.

### CPU features 

* Pipelined execution
* Super scalar execution
* Out of Order Execution using the Tomasulo algorithm

### Planned CPU features
* Better support for different data types
* Speculative execution
* Fences
* SMT (aka hyper-threading)
* CMP (aka multicore)
* Working cache (MESI based)

## Supported instructions

* ADD
* SUB
* MUL
* SDIV
* LDR
* STR
* NOP
* PRINTR
* MOV
* B
* BX
* BL
* CBZ
* CBNZ
* NEG
* AND
* ORR
* EOR

More instructions will be added over time.