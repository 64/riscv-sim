decode:                             
        addi    sp, sp, -16
        sw      ra, 12(sp)                      
        mv      a1, a0
        li      a0, 1000
        mv      a2, sp
        li      a3, 4
        call    qoi_decode
        lw      ra, 12(sp)                      
        addi    sp, sp, 16
        j .end

qoi_decode:          
        addi    sp, sp, -304
        sw      ra, 300(sp)                     
        sw      s0, 296(sp)                     
        sw      s1, 292(sp)                     
        sw      s2, 288(sp)                     
        sw      s3, 284(sp)                     
        sw      s4, 280(sp)                     
        sw      s5, 276(sp)                     
        sw      s6, 272(sp)                     
        sw      s7, 268(sp)                     
        sw      s8, 264(sp)                     
        mv      s0, a0
        seqz    a0, a0
        seqz    a4, a2
        or      a4, a0, a4
        li      a0, 0
        bnez    a4, .LBB0_6
        mv      s1, a1
        addi    a0, a3, -3
        snez    a0, a0
        andi    a1, a3, -5
        snez    a1, a1
        and     a0, a0, a1
        slti    a1, s1, 22
        or      a1, a1, a0
        li      a0, 0
        bnez    a1, .LBB0_6
        lb      a7, 0(s0)
        lbu     t0, 1(s0)
        lbu     a6, 2(s0)
        lbu     a5, 3(s0)
        lb      a0, 4(s0)
        lbu     a1, 5(s0)
        lbu     a4, 6(s0)
        lbu     t1, 7(s0)
        slli    a0, a0, 24
        slli    a1, a1, 16
        or      a0, a1, a0
        slli    a1, a4, 8
        or      a0, a0, a1
        or      a1, a0, t1
        sw      a1, 0(a2)
        lb      a0, 8(s0)
        lbu     a4, 9(s0)
        lbu     t1, 10(s0)
        lbu     t2, 11(s0)
        slli    a0, a0, 24
        slli    a4, a4, 16
        or      a0, a4, a0
        slli    a4, t1, 8
        or      a0, a0, a4
        or      a4, a0, t2
        sw      a4, 4(a2)
        lbu     s2, 12(s0)
        li      a0, 0
        sb      s2, 8(a2)
        lbu     t1, 13(s0)
        seqz    t2, a1
        seqz    t3, a4
        or      t2, t2, t3
        sb      t1, 9(a2)
        bnez    t2, .LBB0_6
        addi    a0, s2, -5
        andi    a0, a0, 255
        li      a2, 254
        bgeu    a0, a2, .LBB0_5
.LBB0_4:
        li      a0, 0
        j       .LBB0_6
.LBB0_5:
        slli    a0, a7, 24
        slli    a2, t0, 16
        or      a0, a2, a0
        slli    a2, a6, 8
        or      a0, a0, a2
        or      a0, a0, a5
        sltiu   a2, t1, 2
        xori    a2, a2, 1
        lui     a5, 464631
        addi    a5, a5, -1690
        xor     a0, a0, a5
        snez    a0, a0
        or      a2, a0, a2
        li      a0, 0
        beqz    a2, .LBB0_7
.LBB0_6:
        lw      ra, 300(sp)                     
        lw      s0, 296(sp)                     
        lw      s1, 292(sp)                     
        lw      s2, 288(sp)                     
        lw      s3, 284(sp)                     
        lw      s4, 280(sp)                     
        lw      s5, 276(sp)                     
        lw      s6, 272(sp)                     
        lw      s7, 268(sp)                     
        lw      s8, 264(sp)                     
        addi    sp, sp, 304
        ret
.LBB0_7:
        lui     a0, 97656
        addi    a0, a0, 1024
        divu    a0, a0, a1
        bgeu    a4, a0, .LBB0_4
        beqz    a3, .LBB0_10
        mv      s2, a3
.LBB0_10:
        mul     a0, a4, a1
        mul     s3, a0, s2
        addi    a0, sp, 8
        li      a2, 256
        addi    s4, sp, 8
        li      a1, 0
        call    q_memset
        lui     a1, 122
        blez    s3, .LBB0_31
        li      s5, 0
        li      a2, 0
        li      t4, 0
        li      t3, 0
        li      t2, 0
        addi    a3, s1, -8
        li      t6, 14
        li      a4, 255
        addi    a0, a1, 288
        li      a5, 4
        li      a6, 11
        li      a7, 254
        li      t0, 1
        li      t1, 2
        li      t5, 255
        j       .LBB0_13
.LBB0_12:                               
        add     a2, a2, s2
        bge     a2, s3, .LBB0_6
.LBB0_13:                               
        blez    s5, .LBB0_15
        addi    s5, s5, -1
        j       .LBB0_29
.LBB0_15:                               
        bge     t6, a3, .LBB0_19
        add     s5, s0, t6
        lbu     s6, 0(s5)
        addi    s1, t6, 1
        beq     s6, a4, .LBB0_20
        bne     s6, a7, .LBB0_21
        add     t2, s0, s1
        lbu     t2, 0(t2)
        lbu     t3, 2(s5)
        lbu     t4, 3(s5)
        li      s5, 0
        addi    s1, t6, 4
        j       .LBB0_28
.LBB0_19:                               
        li      s5, 0
        j       .LBB0_29
.LBB0_20:                               
        add     t2, s0, s1
        lbu     t2, 0(t2)
        lbu     t3, 2(s5)
        lbu     t4, 3(s5)
        lbu     t5, 4(s5)
        li      s5, 0
        addi    s1, t6, 5
        j       .LBB0_28
.LBB0_21:                               
        srli    s5, s6, 6
        blt     t0, s5, .LBB0_24
        bnez    s5, .LBB0_26
        slli    t2, s6, 2
        add     t5, s4, t2
        lbu     t2, 0(t5)
        ori     t3, t5, 1
        lbu     t3, 0(t3)
        ori     t4, t5, 2
        lbu     t4, 0(t4)
        ori     t5, t5, 3
        lbu     t5, 0(t5)
        j       .LBB0_28
.LBB0_24:                               
        bne     s5, t1, .LBB0_27
        add     s1, s0, s1
        lbu     s1, 0(s1)
        li      s5, 0
        andi    s6, s6, 63
        addi    s7, s6, -40
        srli    s8, s1, 4
        add     s8, s8, s7
        add     t2, t2, s8
        addi    t6, t6, 2
        add     t3, t3, s6
        addi    t3, t3, -32
        andi    s1, s1, 15
        add     s1, s1, s7
        add     t4, t4, s1
        mv      s1, t6
        j       .LBB0_28
.LBB0_26:                               
        li      s5, 0
        slli    t6, s6, 26
        srli    t6, t6, 30
        add     t2, t2, t6
        addi    t2, t2, -2
        slli    t6, s6, 28
        srli    t6, t6, 30
        add     t3, t3, t6
        addi    t3, t3, -2
        andi    t6, s6, 3
        add     t4, t4, t6
        addi    t4, t4, -2
        j       .LBB0_28
.LBB0_27:                               
        andi    s5, s6, 63
.LBB0_28:                               
        slli    t6, t2, 1
        add     t6, t6, t2
        slli    s6, t3, 2
        add     s6, s6, t3
        add     t6, s6, t6
        slli    s6, t4, 3
        sub     s6, s6, t4
        add     t6, t6, s6
        mul     s6, t5, a6
        add     t6, t6, s6
        andi    t6, t6, 63
        slli    t6, t6, 2
        add     t6, s4, t6
        sb      t2, 0(t6)
        ori     s6, t6, 1
        sb      t3, 0(s6)
        ori     s6, t6, 2
        sb      t4, 0(s6)
        ori     t6, t6, 3
        sb      t5, 0(t6)
        mv      t6, s1
.LBB0_29:                               
        add     s1, a2, a0
        sb      t2, 0(s1)
        sb      t3, 1(s1)
        sb      t4, 2(s1)
        bne     s2, a5, .LBB0_12
        add     s1, a2, a1
        sb      t5, 291(s1)
        j       .LBB0_12
.LBB0_31:
        addi    a0, a1, 288
        j       .LBB0_6

q_memset:
        beq     a2,zero,.L48
        add     a2,a0,a2
        sb      a1,0(a2)
.L48:
        ret
	
.end:
		nop
