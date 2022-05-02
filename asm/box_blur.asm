; Compile with -O3 -mabi=ilp32 -march=rv32im
; void box_blur(int *in_data, int dims) {
;     int *out_data = (int *)500000;
; 
;     for (int x = 0; x < dims; x++) {
;         for (int y = 0; y < dims; y++) {
;             int count = 0, total = 0;
;             
;             for (int xn = x - 1; xn <= x + 1; xn++) {
;                 for (int yn = y - 1; yn <= y + 1; yn++) {
;                     if ((0 <= xn && xn < dims) && (0 <= yn && yn < dims)) {
;                         total += in_data[y * dims + x];
;                         count++;    
;                     }
;                 }
;             }
; 
;             out_data[y * dims + x] = total / count;
;         }
;     }
; }

main:
	call box_blur
	j .end

box_blur:                               
        addi    sp, sp, -16
        sw      s0, 12(sp)                      
        sw      s1, 8(sp)                       
        blez    a1, .LBB0_26
        li      t0, 0
        slli    a2, a1, 2
        lui     a3, 122
        addi    a4, a3, 288
        add     a4, a2, a4
        add     a5, a0, a2
        li      a6, 2
        li      a7, 1
        j       .LBB0_3
.LBB0_2:                                
        addi    a4, a4, 4
        addi    a5, a5, 4
        mv      t0, t1
        beq     t1, a1, .LBB0_26
.LBB0_3:                                
        slli    t2, t0, 2
        add     t3, a0, t2
        beqz    t0, .LBB0_8
        lw      t4, 0(t3)
        li      t1, 1
        blt     a1, a6, .LBB0_6
        lw      t1, 0(t3)
        add     t4, t1, t4
        li      t1, 2
.LBB0_6:                                
        lw      t5, 0(t3)
        add     t4, t5, t4
        blt     a1, a6, .LBB0_9
.LBB0_7:                                
        lw      t5, 0(t3)
        add     t4, t5, t4
        addi    t5, t1, 2
        addi    t1, t0, 1
        blt     t1, a1, .LBB0_10
        j       .LBB0_12
.LBB0_8:                                
        li      t1, 0
        li      t4, 0
        lw      t5, 0(t3)
        add     t4, t5, t4
        bge     a1, a6, .LBB0_7
.LBB0_9:                                
        addi    t5, t1, 1
        addi    t1, t0, 1
        bge     t1, a1, .LBB0_12
.LBB0_10:                               
        lw      t6, 0(t3)
        add     t4, t6, t4
        blt     a1, a6, .LBB0_25
        lw      t3, 0(t3)
        add     t4, t3, t4
        addi    t5, t5, 2
.LBB0_12:                               
        div     t3, t4, t5
        add     t2, t2, a3
        sw      t3, 288(t2)
        beq     a1, a7, .LBB0_2
.LBB0_13:                               
        li      t2, 0
        li      t5, 1
        j       .LBB0_16
.LBB0_14:                               
        lw      t3, 0(t3)
        addi    t4, s0, 1
        add     t6, t3, t6
        addi    s0, t4, 2
.LBB0_15:                               
        div     t3, t6, s0
        add     t4, a4, t2
        sw      t3, 0(t4)
        add     t2, t2, a2
        beq     a1, t5, .LBB0_2
.LBB0_16:                               
        mv      t4, t5
        add     t3, a5, t2
        beqz    t0, .LBB0_19
        lw      t5, 0(t3)
        add     t5, t5, t5
        addi    t6, t4, 1
        li      s0, 2
        bge     t6, a1, .LBB0_20
        lw      t6, 0(t3)
        add     t5, t6, t5
        li      s0, 3
        j       .LBB0_20
.LBB0_19:                               
        li      s0, 0
        li      t5, 0
.LBB0_20:                               
        lw      t6, 0(t3)
        add     s1, t6, t5
        addi    t5, t4, 1
        add     t6, t6, s1
        bge     t5, a1, .LBB0_22
        lw      s1, 0(t3)
        addi    s0, s0, 1
        add     t6, s1, t6
.LBB0_22:                               
        addi    s0, s0, 2
        bge     t1, a1, .LBB0_15
        lw      s1, 0(t3)
        add     t6, s1, t6
        addi    t4, t4, 1
        add     t6, s1, t6
        blt     t4, a1, .LBB0_14
        addi    s0, s0, 2
        j       .LBB0_15
.LBB0_25:                               
        addi    t5, t5, 1
        div     t3, t4, t5
        add     t2, t2, a3
        sw      t3, 288(t2)
        beq     a1, a7, .LBB0_2
        j       .LBB0_13
.LBB0_26:
        lw      s0, 12(sp)                      
        lw      s1, 8(sp)                       
        addi    sp, sp, 16
        ret

.end:
		nop
