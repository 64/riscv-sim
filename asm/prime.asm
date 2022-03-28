; compile with -O3 -march=rv32im -mabi=ilp32
;
; int is_prime(int x) {
;     for (int i = 2; i < x; i++) {
;         if (x % i == 0) {
;             return 0;
;         }
;     }
;     return 1;
; }

is_prime:
        li      a5,2
        mv      a4,a0
        ble     a0,a5,.L5
        andi    a5,a0,1
        beq     a5,zero,.L6
        li      a5,2
        j       .L3
.L4:
        rem     a0,a4,a5
        beq     a0,zero,.L1
.L3:
        addi    a5,a5,1
        bne     a4,a5,.L4
.L5:
        li      a0,1
        j .L1 ; ret
.L6:
        li      a0,0
.L1:
        ; ret
