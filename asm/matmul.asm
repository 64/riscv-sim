; Compile with -O3 -march=rv32im -mabi=ilp32
;
; static void matmul(int * restrict a, int * restrict b, int * restrict out, int dim) {
;     for (int i = 0; i < dim; i++) {
;         for (int j = 0; j < dim; j++) {
;             for (int k = 0; k < dim; k++) {
;                 out[j * dim + i] += a[j * dim + k] * b[k * dim + i];
;             }
;         }
;     }
; }
; 
; void generate(int *start, int dim) {
;     int *a = start;
;     int *b = start + dim * dim;
;     int *c = start + (dim * dim * 2);
;     
;     for (int i = 0; i < dim; i++) {
;         for (int j = 0; j < dim; j++) {
;             int val;
;             if (i == j) {
;                 val = 1;
;             } else {
;                 val = 0;
;             }
; 
;             a[j * dim + i] = b[j * dim + i] = val;
;             c[j * dim + i] = 0;
;         }
;     }
; 
;     matmul(a, b, c, dim);
; }

generate:
        mul     t5,a1,a1
        mv      t4,a0
        slli    t5,t5,2
        add     t0,a0,t5
        add     t5,t0,t5
        ble     a1,zero,.L19
        addi    sp,sp,-16
        sw      s0,12(sp)
        sw      s1,8(sp)
        mv      a6,a1
        slli    a0,a1,2
        li      t3,0
        li      a7,0
.L5:
        mv      t6,t0
        add     a1,t0,t3
        add     a2,t4,t3
        mv      t2,t5
        add     a3,t5,t3
        li      a4,0
.L4:
        sub     a5,a7,a4
        seqz    a5,a5
        sw      a5,0(a1)
        sw      a5,0(a2)
        mv      t1,a4
        sw      zero,0(a3)
        addi    a4,a4,1
        add     a1,a1,a0
        add     a2,a2,a0
        add     a3,a3,a0
        bne     a6,a4,.L4
        addi    a5,a7,1
        addi    t3,t3,4
        beq     a7,t1,.L20
        mv      a7,a5
        j       .L5
.L20:
        neg     a6,a6
        add     s1,t4,a0
        slli    t0,a6,2
        li      s0,0
        slli    t5,a6,3
.L6:
        mv      a6,s1
        mv      a7,t2
        li      t3,0
.L9:
        lw      a2,0(a7)
        add     t4,t0,a6
        mv      a3,t6
        mv      a5,t4
.L7:
        lw      a4,0(a5)
        lw      a1,0(a3)
        addi    a5,a5,4
        add     a3,a3,a0
        mul     a4,a4,a1
        add     a2,a2,a4
        bne     a6,a5,.L7
        sw      a2,0(a7)
        addi    a5,t3,1
        add     a7,a7,a0
        sub     a6,t4,t5
        beq     t1,t3,.L8
        mv      t3,a5
        j       .L9
.L8:
        addi    a5,s0,1
        addi    t6,t6,4
        addi    t2,t2,4
        beq     t1,s0,.L1
        mv      s0,a5
        j       .L6
.L1:
        lw      s0,12(sp)
        lw      s1,8(sp)
        addi    sp,sp,16
        ; jr      ra
.L19:
        ; ret
