; reg / reg
li a0, 1
addi a0, a0, 2
sw a0, 0(zero)

nop
nop
nop
nop
nop

; reg / mem
li a0, 1
li a1, 0

nop
nop
nop
nop
nop

sw a0, 4(zero)
lw a1, 4(zero)

nop
nop
nop
nop
nop

sw a1, 4(zero)
li a0, 0
li a1, 0

nop
nop
nop
nop
nop

; reg / mem
li a0, 1
sw a0, 8(zero)
