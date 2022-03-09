.start1:
	li a1, 4
	addi a0, a0, 1
	nop
	nop
	nop
	bne zero, zero, .start1
	bne a0, a1, .start1

.end1:
	nop
	nop
	nop
	sw a0, 0(zero)
	nop
	nop
	nop
	li a0, 0
	nop
	nop
	nop

.start2:
	li a1, 3
	addi a0, a0, 1
	nop
	nop
	nop
	bne zero, zero, .start2
	nop
	bne a0, a1, .start2

.end2:
	nop
	nop
	nop
	sw a0, 4(zero)
	nop
	nop
	nop
	li a0, 0
	nop
	nop
	nop

.start3:
	li a1, 2
	addi a0, a0, 1
	nop
	nop
	nop
	bne zero, zero, .start3
	nop
	nop
	bne a0, a1, .start3

.end3:
	nop
	nop
	nop
	sw a0, 8(zero)
