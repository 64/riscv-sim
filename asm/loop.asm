; compile with -O3 -march=rv32i -mabi=ilp32
; void loop(int * restrict a, int * restrict b, int * restrict c, int n) {
;     for (int i = 0; i < n; i++) {
;         a[i] = b[i] + c[i];
;     }
; }

loop:
        ble     a3,zero,.L1
		nop
		nop
        slli    a3,a3,2
		nop
		nop
        add     a3,a1,a3
.L3:
        lw      a5,0(a1)
		nop
		nop
        lw      a4,0(a2)
		nop
		nop
        addi    a1,a1,4
		nop
		nop
        addi    a2,a2,4
		nop
		nop
        add     a5,a5,a4
		nop
		nop
        sw      a5,0(a0)
		nop
		nop
        addi    a0,a0,4
		nop
		nop
        bne     a3,a1,.L3
.L1:
        ;ret
