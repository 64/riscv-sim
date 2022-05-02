; Compile with -mabi=ilp32 -march=rv32im
; 
; 
; uint64_t factorial(int x) {
;     if (x == 0) {
;         return 1;
;     } else {
;         return x * factorial(x - 1);
;     }
; }

wrap:
		call factorial
		j .end

factorial:                              
        addi    sp, sp, -32
        sw      ra, 28(sp)                      
        sw      s0, 24(sp)                      
        addi    s0, sp, 32
        sw      a0, -16(s0)
        lw      a0, -16(s0)
        li      a1, 0
        bne     a0, a1, .LBB0_2
        j       .LBB0_1
.LBB0_1:
        li      a0, 1
        sw      a0, -12(s0)
        j       .LBB0_3
.LBB0_2:
        lw      a0, -16(s0)
        sw      a0, -20(s0)                     
        addi    a0, a0, -1
        call    factorial
        mv      a1, a0
        lw      a0, -20(s0)                     
        mul     a0, a0, a1
        sw      a0, -12(s0)
        j       .LBB0_3
.LBB0_3:
        lw      a0, -12(s0)
        lw      ra, 28(sp)                      
        lw      s0, 24(sp)                      
        addi    sp, sp, 32
        ret

.end:
		nop
