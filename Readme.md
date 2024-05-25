# Rust based ARM CPU emulator

## About The Project
The primary aim of this project is to provide a possible implementation of modern
processors in software (Rust) for the ARM instruction set.

The goal of the project is to provide some insights in how modern processors potentially
could work.

The goal of the project is not to provide a super fast implementation; for that
code generation using binary translation would be orders of magnitude faster.  

## Warning

This project is toy project. I want to upgrade my Rust skills and I needed a serious
enough challenge to work on. Also the challenge should initially be without the need 
for concurrency control so that I get a better understanding of ownership. 

### CPU features 

* Pipelined execution
* Super scalar execution
* Out of Order Execution using Tomasulo's algorithm. So only RAW dependencies are preserved.
* Store buffer
* Performance monitor (although not exposed itself through registers).

### Planned CPU features
* Better support for different data types
* Speculative execution
* One way fences like LDAR, STLR, LDAPR. 
* Two way fences like DMB
* Serializing instructions like DSB
* Exclusive access instructions like LDXR, STXR, LDAXR, STLXR
* SMT (aka hyper-threading)
* CMP (aka multicore)
* Working cache (MESI based)
* Write coalescing
* Store buffer out of order commit to the cache

## Supported instructions

* ADD
* SUB
* MUL
* SDIV
* LDR
* STR
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
* MVN
* NOP
* CMP
* BEQ
* BNE
* BLE
* BLT
* BGE
* BGT

And some none official ones:
* PRINTR: prints the value of a register.

More instructions will be added over time.

## How to run

```bash
cargo run -- --file asm/high_ipc.asm --config cpu.yaml
```

