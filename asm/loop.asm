// Computes:
// for (int i = 0; i < 10; i++) {
//     a[i] = b[i] + c[i];
// }

	li r6, 40

.begin:
	add r10, r1, r5
	lw r15, [r10]
	
	add r10, r2, r5
	lw r14, [r10]

	add r13, r14, r15
	add r10, r0, r5
	sw [r10], r13

	addi r5, r5, 4
	bne r5, r6, .begin
.end:
