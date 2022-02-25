// Computes:
// for (int i = 0; i < 10; i++) {
//     a[i] = b[i] + c[i];
// }

	li t6, 40

.begin:
	add t1, a1, t0
	lw t2, 0(t1)
	
	add t1, a2, t0
	lw t3, 0(t1)

	add t4, t2, t3
	add t1, a0, t0
	sw 0(t1), t4

	addi t0, t0, 4
	bne t0, t6, .begin
.end:
