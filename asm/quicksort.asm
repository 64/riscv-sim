; Compile with -O3 -march=rv32im -mabi=ilp32
; void quickSort(int arr[], int start, int end);
; 
; void generate(int *data, int size) {
;     for (int i = 0; i < size; i++) {
;         data[i] = size - i;
;     }
;     quickSort(data, 0, size - 1);
; }
; 
; static void swap(int *x, int *y)
; {
;     int t = *x;
;     *x = *y;
;     *y = t;
; }
; 
; static int partition(int arr[], int start, int end)
; {
;     int pIndex = start;
;     int pivot = arr[end];
;     int i;
;     for(i = start; i < end; i++)
;     {
;         if(arr[i] < pivot)
;         {
;             swap(&arr[i], &arr[pIndex]);
;             pIndex++;
;         }
;     }
;     swap(&arr[end], &arr[pIndex]);
;     return pIndex;
; }
; 
; void quickSort(int arr[], int start, int end)
; {
;     if(start < end)
;     {
;         int pIndex = partition(arr, start, end);
;         quickSort(arr, start, pIndex-1);
;         quickSort(arr, pIndex+1, end);
;     }
; }


generate:
        addi    sp,sp,-32
        sw      s1,20(sp)
        sw      ra,28(sp)
        sw      s0,24(sp)
        sw      s2,16(sp)
        sw      s3,12(sp)
        mv      s1,a0
        mv      a5,a1
        mv      a4,a0
        ble     a1,zero,.L95
.L94:
        sw      a5,0(a4)
        addi    a5,a5,-1
        addi    a4,a4,4
        bne     a5,zero,.L94
.L95:
        addi    s2,a1,-1
        ble     s2,zero,.L90
        slli    s3,s2,2
        add     s3,s1,s3
        li      a1,0
.L98:
        slli    a4,a1,2
        add     a4,s1,a4
        lw      t3,0(s3)
        lw      t1,0(a4)
        mv      a2,a1
        mv      a3,a1
.L97:
        lw      a6,0(a4)
        slli    a5,a2,2
        addi    a7,a5,4
        addi    a3,a3,1
        addi    a0,a2,2
        add     a5,s1,a5
        addi    s0,a2,1
        ble     t3,a6,.L96
        sw      t1,0(a4)
        sw      a6,0(a5)
        add     a5,s1,a7
        lw      t1,0(a5)
        mv      a2,s0
        mv      s0,a0
.L96:
        addi    a4,a4,4
        bgt     s2,a3,.L97
        lw      a4,0(s3)
        sw      t1,0(s3)
        addi    a2,a2,-1
        sw      a4,0(a5)
        mv      a0,s1
        call    quickSort
        ble     s2,s0,.L90
        mv      a1,s0
        j       .L98
.L90:
        lw      ra,28(sp)
        lw      s0,24(sp)
        lw      s1,20(sp)
        lw      s2,16(sp)
        lw      s3,12(sp)
        addi    sp,sp,32
        j .end

quickSort:
        slli    a5,a2,2
        addi    sp,sp,-128
        add     a5,a0,a5
        sw      ra,124(sp)
        sw      s0,120(sp)
        sw      s1,116(sp)
        sw      s2,112(sp)
        sw      s3,108(sp)
        sw      s4,104(sp)
        sw      s5,100(sp)
        sw      s6,96(sp)
        sw      s7,92(sp)
        sw      s8,88(sp)
        sw      s9,84(sp)
        sw      s10,80(sp)
        sw      s11,76(sp)
        sw      a2,32(sp)
        sw      a5,4(sp)
        ble     a2,a1,.L1
        mv      s7,a0
        mv      a7,a1
.L6:
        lw      a5,4(sp)
        slli    t2,a7,2
        add     a4,s7,t2
        lw      a6,0(a5)
        lw      a2,0(a4)
        mv      a3,a7
        mv      a1,a7
.L4:
        lw      t1,0(a4)
        slli    a5,a3,2
        addi    t4,a5,4
        addi    a1,a1,1
        addi    t3,a3,2
        add     a5,s7,a5
        addi    a0,a3,1
        ble     a6,t1,.L3
        sw      a2,0(a4)
        sw      t1,0(a5)
        add     a5,s7,t4
        lw      a2,0(a5)
        mv      a3,a0
        mv      a0,t3
.L3:
        lw      t1,32(sp)
        addi    a4,a4,4
        bgt     t1,a1,.L4
        lw      a4,4(sp)
        addi    a3,a3,-1
        sw      a0,24(sp)
        lw      a1,0(a4)
        sw      a2,0(a4)
        slli    a4,a3,2
        sw      a1,0(a5)
        add     a5,s7,a4
        sw      a3,0(sp)
        sw      a5,12(sp)
        mv      t0,s7
        mv      s6,a7
        ble     a3,a7,.L12
.L11:
        lw      a4,12(sp)
        add     a5,t0,t2
        lw      a2,0(a5)
        lw      a6,0(a4)
        mv      a3,s6
        mv      a1,s6
.L9:
        lw      a7,0(a5)
        slli    a4,a3,2
        addi    t3,a4,4
        addi    a1,a1,1
        addi    t1,a3,2
        add     a4,t0,a4
        addi    a0,a3,1
        ble     a6,a7,.L8
        sw      a2,0(a5)
        sw      a7,0(a4)
        add     a4,t0,t3
        lw      a2,0(a4)
        mv      a3,a0
        mv      a0,t1
.L8:
        lw      a7,0(sp)
        addi    a5,a5,4
        bgt     a7,a1,.L9
        lw      a5,12(sp)
        addi    a3,a3,-1
        sw      a0,28(sp)
        lw      a1,0(a5)
        sw      a2,0(a5)
        slli    a5,a3,2
        add     a5,t0,a5
        sw      a1,0(a4)
        sw      a5,16(sp)
        ble     a3,s6,.L17
        sw      a3,36(sp)
        mv      s0,t0
        mv      a1,s6
.L16:
        lw      a4,16(sp)
        add     a5,s0,t2
        lw      a0,0(a5)
        lw      t1,0(a4)
        mv      a2,a1
        mv      a6,a1
.L14:
        lw      a7,0(a5)
        slli    a4,a2,2
        addi    t4,a4,4
        addi    a6,a6,1
        addi    t3,a2,2
        add     a4,s0,a4
        addi    a3,a2,1
        ble     t1,a7,.L13
        sw      a0,0(a5)
        sw      a7,0(a4)
        add     a4,s0,t4
        lw      a0,0(a4)
        mv      a2,a3
        mv      a3,t3
.L13:
        lw      a7,36(sp)
        addi    a5,a5,4
        bgt     a7,a6,.L14
        lw      a5,16(sp)
        addi    a2,a2,-1
        sw      a2,8(sp)
        lw      a6,0(a5)
        sw      a0,0(a5)
        slli    a5,a2,2
        add     a5,s0,a5
        sw      a6,0(a4)
        sw      a5,20(sp)
        ble     a2,a1,.L22
.L21:
        lw      a4,20(sp)
        add     a5,s0,t2
        lw      a2,0(a5)
        lw      a7,0(a4)
        mv      s1,a1
        mv      a0,a1
.L19:
        lw      a6,0(a5)
        slli    a4,s1,2
        addi    t3,a4,4
        addi    a0,a0,1
        addi    t1,s1,2
        add     a4,s0,a4
        addi    s11,s1,1
        ble     a7,a6,.L18
        sw      a2,0(a5)
        sw      a6,0(a4)
        add     a4,s0,t3
        lw      a2,0(a4)
        mv      s1,s11
        mv      s11,t1
.L18:
        lw      a6,8(sp)
        addi    a5,a5,4
        bgt     a6,a0,.L19
        lw      a0,20(sp)
        addi    s1,s1,-1
        slli    s6,s1,2
        lw      a5,0(a0)
        sw      a2,0(a0)
        add     s6,s0,s6
        sw      a5,0(a4)
        mv      s3,a3
        ble     s1,a1,.L27
.L26:
        add     a5,s0,t2
        lw      a6,0(s6)
        lw      a3,0(a5)
        mv      s2,a1
        mv      a2,a1
.L24:
        lw      a0,0(a5)
        slli    a4,s2,2
        addi    t1,a4,4
        addi    a2,a2,1
        addi    a7,s2,2
        add     a4,s0,a4
        addi    s4,s2,1
        ble     a6,a0,.L23
        sw      a3,0(a5)
        sw      a0,0(a4)
        add     a4,s0,t1
        lw      a3,0(a4)
        mv      s2,s4
        mv      s4,a7
.L23:
        addi    a5,a5,4
        bgt     s1,a2,.L24
        lw      a5,0(s6)
        addi    s2,s2,-1
        sw      a3,0(s6)
        slli    s5,s2,2
        sw      a5,0(a4)
        add     s5,s0,s5
        ble     s2,a1,.L32
.L31:
        add     a5,s0,t2
        lw      a7,0(s5)
        lw      a2,0(a5)
        mv      a3,a1
        mv      a0,a1
.L29:
        lw      a6,0(a5)
        slli    a4,a3,2
        addi    t3,a4,4
        addi    a0,a0,1
        addi    t1,a3,2
        add     a4,s0,a4
        addi    s8,a3,1
        ble     a7,a6,.L28
        sw      a2,0(a5)
        sw      a6,0(a4)
        add     a4,s0,t3
        lw      a2,0(a4)
        mv      a3,s8
        mv      s8,t1
.L28:
        addi    a5,a5,4
        bgt     s2,a0,.L29
        lw      a5,0(s5)
        addi    s7,a3,-1
        sw      a2,0(s5)
        slli    s10,s7,2
        sw      a5,0(a4)
        add     s10,s0,s10
        ble     s7,a1,.L37
.L36:
        add     a5,s0,t2
        lw      t1,0(s10)
        lw      a2,0(a5)
        mv      a3,a1
        mv      a0,a1
.L34:
        lw      a7,0(a5)
        slli    a4,a3,2
        addi    t4,a4,4
        addi    a0,a0,1
        addi    t3,a3,2
        add     a4,s0,a4
        addi    a6,a3,1
        ble     t1,a7,.L33
        sw      a2,0(a5)
        sw      a7,0(a4)
        add     a4,s0,t4
        lw      a2,0(a4)
        mv      a3,a6
        mv      a6,t3
.L33:
        addi    a5,a5,4
        bgt     s7,a0,.L34
        lw      a5,0(s10)
        addi    s9,a3,-1
        sw      a2,0(s10)
        slli    a7,s9,2
        sw      a5,0(a4)
        add     a7,s0,a7
        ble     s9,a1,.L42
        mv      a0,s0
        mv      s0,a7
.L41:
        add     a5,a0,t2
        lw      t4,0(s0)
        lw      t3,0(a5)
        mv      a3,a1
        mv      a2,a1
.L39:
        lw      t1,0(a5)
        slli    a4,a3,2
        addi    t6,a4,4
        addi    a2,a2,1
        addi    t5,a3,2
        add     a4,a0,a4
        addi    a7,a3,1
        ble     t4,t1,.L38
        sw      t3,0(a5)
        sw      t1,0(a4)
        add     a4,a0,t6
        lw      t3,0(a4)
        mv      a3,a7
        mv      a7,t5
.L38:
        addi    a5,a5,4
        bgt     s9,a2,.L39
        lw      a5,0(s0)
        addi    a3,a3,-1
        sw      t3,0(s0)
        slli    t1,a3,2
        sw      a5,0(a4)
        add     t1,a0,t1
        ble     a3,a1,.L46
        mv      a4,s4
        mv      t3,s8
        mv      s4,s2
        mv      s8,s0
        mv      s2,a3
        mv      a5,t2
        mv      a3,s11
        mv      s11,s3
        mv      s3,s1
        mv      s1,t1
        mv      t1,a4
.L45:
        add     a5,a0,a5
        lw      t0,0(s1)
        lw      t4,0(a5)
        mv      a2,a1
        mv      t5,a1
.L44:
        lw      t6,0(a5)
        slli    a4,a2,2
        addi    ra,a4,4
        addi    t5,t5,1
        addi    t2,a2,2
        add     a4,a0,a4
        addi    s0,a2,1
        ble     t0,t6,.L43
        sw      t4,0(a5)
        sw      t6,0(a4)
        add     a4,a0,ra
        lw      t4,0(a4)
        mv      a2,s0
        mv      s0,t2
.L43:
        addi    a5,a5,4
        bgt     s2,t5,.L44
        lw      a5,0(s1)
        sw      t4,0(s1)
        addi    a2,a2,-1
        sw      a5,0(a4)
        sw      a3,60(sp)
        sw      t1,56(sp)
        sw      t3,52(sp)
        sw      a6,48(sp)
        sw      a7,44(sp)
        sw      a0,40(sp)
        call    quickSort
        lw      a0,40(sp)
        lw      a7,44(sp)
        lw      a6,48(sp)
        lw      t3,52(sp)
        lw      t1,56(sp)
        lw      a3,60(sp)
        ble     s2,s0,.L85
        mv      a1,s0
        slli    a5,s0,2
        j       .L45
.L1:
        lw      ra,124(sp)
        lw      s0,120(sp)
        lw      s1,116(sp)
        lw      s2,112(sp)
        lw      s3,108(sp)
        lw      s4,104(sp)
        lw      s5,100(sp)
        lw      s6,96(sp)
        lw      s7,92(sp)
        lw      s8,88(sp)
        lw      s9,84(sp)
        lw      s10,80(sp)
        lw      s11,76(sp)
        addi    sp,sp,128
        jr      ra
.L89:
        mv      s7,t0
.L12:
        lw      a5,32(sp)
        lw      a4,24(sp)
        ble     a5,a4,.L1
        mv      a7,a4
        j       .L6
.L85:
        mv      s1,s3
        mv      s2,s4
        mv      s0,s8
        mv      s3,s11
        mv      s4,t1
        mv      s11,a3
        mv      s8,t3
.L46:
        ble     s9,a7,.L86
        mv      a1,a7
        slli    t2,a7,2
        j       .L41
.L32:
        ble     s1,s4,.L87
        mv      a1,s4
        slli    t2,s4,2
        j       .L26
.L37:
        ble     s2,s8,.L32
        mv      a1,s8
        slli    t2,s8,2
        j       .L31
.L86:
        mv      s0,a0
.L42:
        ble     s7,a6,.L37
        mv      a1,a6
        slli    t2,a6,2
        j       .L36
.L87:
        mv      a3,s3
.L27:
        lw      a5,8(sp)
        ble     a5,s11,.L22
        mv      a1,s11
        slli    t2,s11,2
        j       .L21
.L22:
        lw      a5,36(sp)
        ble     a5,a3,.L88
        mv      a1,a3
        slli    t2,a3,2
        j       .L16
.L88:
        mv      t0,s0
.L17:
        lw      a5,0(sp)
        lw      a4,28(sp)
        ble     a5,a4,.L89
        mv      s6,a4
        slli    t2,a4,2
        j       .L11

.end:
		nop
