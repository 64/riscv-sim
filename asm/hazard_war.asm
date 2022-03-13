li a0, 1
nop
nop
nop
nop
nop
mv a1, a0
li a0, 2
sw a1, 0(zero)

nop
nop
nop
nop
nop

; mem / mem
li a0, 1
li a1, 2
sw a0, 4(zero)

nop
nop
nop
nop
nop

lw a0, 4(zero)
sw a1, 4(zero)
