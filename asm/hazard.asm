; Registers:
; RAW hazard
li a0, 1
addi a0, a0, 2
sw a0, 0(zero)

nop
nop
nop
nop
nop

; WAW hazard
li a0, 1
li a0, 2
sw a0, 4(zero)

nop
nop
nop
nop
nop

; WAR hazard
li a0, 1
nop
nop
nop
nop
nop
mv a1, a0
li a0, 2
sw a1, 8(zero)

nop
nop
nop
nop
nop

; Memory:
; RAW hazard (mem / mem)
li a0, 1
li a1, 0

nop
nop
nop
nop
nop

sw a0, 12(zero)
lw a1, 12(zero)

nop
nop
nop
nop
nop

sw a1, 12(zero)
li a0, 0
li a1, 0

nop
nop
nop
nop
nop

; RAW hazard (reg / mem)
li a0, 1
sw a0, 16(zero)

nop
nop
nop
nop
nop

; WAW hazard (mem / mem)
li a0, 1
li a1, 2
nop
nop
nop
nop
nop

sw a0, 20(zero)
sw a1, 20(zero)

nop
nop
nop
nop
nop

; WAR hazard (mem / mem)
li a0, 1
li a1, 2
sw a0, 24(zero)
nop
nop
nop
nop
nop

lw a0, 24(zero)
sw a1, 24(zero)
