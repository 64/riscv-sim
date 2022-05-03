# CPU Cycle Simulator: UoB Advanced Computer Architecture

Requires a working Rust toolchain.

For tests, run `cargo test`.

To run an application `asm/foo.asm`, use `$ cargo run --release -- foo [param1] [param2]` where
`param1` and `param2` will be passed to the function in the A0 and A1 registers. If a path is
provided for one of these arguments, a file will be loaded to address 1000 with the contents.

Example:
```
$ cargo run --release -- prime 2946901
   Compiling aca v0.1.0
    Finished release [optimized] target(s) in 2.97s
     Running `target/release/aca prime 2946901`
    EXECUTION COMPLETED
  =======================
              R/S stalls: 2946853
      Direct mispredicts: 0.00% (1/5893799)
    Instructions retired: 11787603
            Cycles taken: 5893808
  Instructions per clock: 2.00
  Simulator time elapsed: 9.74s (605 KHz)
          EU utilisation:
                 Branch = 100%
              LoadStore =  0%
                    Alu = 67%
                    Alu = 33%
```


