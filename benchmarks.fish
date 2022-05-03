#!/bin/fish
cargo run --release -- box_blur 1000 300
cargo run --release -- quicksort 0 500
cargo run --release -- fibonnaci 18
cargo run --release -- matmul 0 50
cargo run --release -- prime 2946901
cargo run --release -- qoi_decode data/riscv-300x300.qoi (wc -c data/riscv-300x300.qoi)
cargo run --release -- qoi_decode_clang data/riscv-300x300.qoi (wc -c data/riscv-300x300.qoi)
