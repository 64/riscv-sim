// Computes:
// for (int i = 0; i < 10; i++) {
//     a[i] = b[i] + c[i];
// }

	loadi r6, 40

.begin:
	add r10, r1, r5
	loadw r15, [r10]
	
	add r10, r2, r5
	loadw r14, [r10]

	add r13, r14, r15
	add r10, r0, r5
	storew [r10], r13

	addi r5, r5, 4
	jne r5, r6, .begin
.end:
