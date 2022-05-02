; Grabbed from https://github.com/phoboslab/qoi
; Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
;
; The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
;
; THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

; #define QOI_SRGB   0
; #define QOI_LINEAR 1
; 
; typedef struct {
; 	unsigned int width;
; 	unsigned int height;
; 	unsigned char channels;
; 	unsigned char colorspace;
; } qoi_desc;
; 
; void *qoi_decode(const void *data, int size, qoi_desc *desc, int channels);
; 
; #include <string.h>
; 
; #ifndef QOI_MALLOC
; 	#define QOI_MALLOC(sz) (void *)500000
; 	#define QOI_FREE(p)    
; #endif
; #ifndef QOI_ZEROARR
; 	#define QOI_ZEROARR(a) memset((a),0,sizeof(a))
; #endif
; 
; #define QOI_OP_INDEX  0x00 /* 00xxxxxx */
; #define QOI_OP_DIFF   0x40 /* 01xxxxxx */
; #define QOI_OP_LUMA   0x80 /* 10xxxxxx */
; #define QOI_OP_RUN    0xc0 /* 11xxxxxx */
; #define QOI_OP_RGB    0xfe /* 11111110 */
; #define QOI_OP_RGBA   0xff /* 11111111 */
; 
; #define QOI_MASK_2    0xc0 /* 11000000 */
; 
; #define QOI_COLOR_HASH(C) (C.rgba.r*3 + C.rgba.g*5 + C.rgba.b*7 + C.rgba.a*11)
; #define QOI_MAGIC \
; 	(((unsigned int)'q') << 24 | ((unsigned int)'o') << 16 | \
; 	 ((unsigned int)'i') <<  8 | ((unsigned int)'f'))
; #define QOI_HEADER_SIZE 14
; 
; /* 2GB is the max file size that this implementation can safely handle. We guard
; against anything larger than that, assuming the worst case with 5 bytes per
; pixel, rounded down to a nice clean value. 400 million pixels ought to be
; enough for anybody. */
; #define QOI_PIXELS_MAX ((unsigned int)400000000)
; 
; typedef union {
; 	struct { unsigned char r, g, b, a; } rgba;
; 	unsigned int v;
; } qoi_rgba_t;
; 
; static const unsigned char qoi_padding[8] = {0,0,0,0,0,0,0,1};
; 
; static void qoi_write_32(unsigned char *bytes, int *p, unsigned int v) {
; 	bytes[(*p)++] = (0xff000000 & v) >> 24;
; 	bytes[(*p)++] = (0x00ff0000 & v) >> 16;
; 	bytes[(*p)++] = (0x0000ff00 & v) >> 8;
; 	bytes[(*p)++] = (0x000000ff & v);
; }
; 
; static unsigned int qoi_read_32(const unsigned char *bytes, int *p) {
; 	unsigned int a = bytes[(*p)++];
; 	unsigned int b = bytes[(*p)++];
; 	unsigned int c = bytes[(*p)++];
; 	unsigned int d = bytes[(*p)++];
; 	return a << 24 | b << 16 | c << 8 | d;
; }
; 
; void *qoi_decode(const void *data, int size, qoi_desc *desc, int channels) {
; 	const unsigned char *bytes;
; 	unsigned int header_magic;
; 	unsigned char *pixels;
; 	qoi_rgba_t index[64];
; 	qoi_rgba_t px;
; 	int px_len, chunks_len, px_pos;
; 	int p = 0, run = 0;
; 
; 	if (
; 		data == NULL || desc == NULL ||
; 		(channels != 0 && channels != 3 && channels != 4) ||
; 		size < QOI_HEADER_SIZE + (int)sizeof(qoi_padding)
; 	) {
; 		return NULL;
; 	}
; 
; 	bytes = (const unsigned char *)data;
; 
; 	header_magic = qoi_read_32(bytes, &p);
; 	desc->width = qoi_read_32(bytes, &p);
; 	desc->height = qoi_read_32(bytes, &p);
; 	desc->channels = bytes[p++];
; 	desc->colorspace = bytes[p++];
; 
; 	if (
; 		desc->width == 0 || desc->height == 0 ||
; 		desc->channels < 3 || desc->channels > 4 ||
; 		desc->colorspace > 1 ||
; 		header_magic != QOI_MAGIC ||
; 		desc->height >= QOI_PIXELS_MAX / desc->width
; 	) {
; 		return NULL;
; 	}
; 
; 	if (channels == 0) {
; 		channels = desc->channels;
; 	}
; 
; 	px_len = desc->width * desc->height * channels;
; 	pixels = (unsigned char *) QOI_MALLOC(px_len);
; 	if (!pixels) {
; 		return NULL;
; 	}
; 
; 	QOI_ZEROARR(index);
; 	px.rgba.r = 0;
; 	px.rgba.g = 0;
; 	px.rgba.b = 0;
; 	px.rgba.a = 255;
; 
; 	chunks_len = size - (int)sizeof(qoi_padding);
; 	for (px_pos = 0; px_pos < px_len; px_pos += channels) {
; 		if (run > 0) {
; 			run--;
; 		}
; 		else if (p < chunks_len) {
; 			int b1 = bytes[p++];
; 
; 			if (b1 == QOI_OP_RGB) {
; 				px.rgba.r = bytes[p++];
; 				px.rgba.g = bytes[p++];
; 				px.rgba.b = bytes[p++];
; 			}
; 			else if (b1 == QOI_OP_RGBA) {
; 				px.rgba.r = bytes[p++];
; 				px.rgba.g = bytes[p++];
; 				px.rgba.b = bytes[p++];
; 				px.rgba.a = bytes[p++];
; 			}
; 			else if ((b1 & QOI_MASK_2) == QOI_OP_INDEX) {
; 				px = index[b1];
; 			}
; 			else if ((b1 & QOI_MASK_2) == QOI_OP_DIFF) {
; 				px.rgba.r += ((b1 >> 4) & 0x03) - 2;
; 				px.rgba.g += ((b1 >> 2) & 0x03) - 2;
; 				px.rgba.b += ( b1       & 0x03) - 2;
; 			}
; 			else if ((b1 & QOI_MASK_2) == QOI_OP_LUMA) {
; 				int b2 = bytes[p++];
; 				int vg = (b1 & 0x3f) - 32;
; 				px.rgba.r += vg - 8 + ((b2 >> 4) & 0x0f);
; 				px.rgba.g += vg;
; 				px.rgba.b += vg - 8 +  (b2       & 0x0f);
; 			}
; 			else if ((b1 & QOI_MASK_2) == QOI_OP_RUN) {
; 				run = (b1 & 0x3f);
; 			}
; 
; 			index[QOI_COLOR_HASH(px) % 64] = px;
; 		}
; 
; 		pixels[px_pos + 0] = px.rgba.r;
; 		pixels[px_pos + 1] = px.rgba.g;
; 		pixels[px_pos + 2] = px.rgba.b;
; 		
; 		if (channels == 4) {
; 			pixels[px_pos + 3] = px.rgba.a;
; 		}
; 	}
; 
; 	return pixels;
; }
; 
; void *decode(int size) {
;     qoi_desc desc;
;     return qoi_decode((void *)1000, size, &desc, 4);
; }

decode:
        addi    sp,sp,-32
        mv      a1,a0
        addi    a2,sp,4
        li      a3,4
        li      a0,1000
        sw      ra,28(sp)
        call    qoi_decode
        lw      ra,28(sp)
        addi    sp,sp,32
		j .end

qoi_decode:
        beq     a0,zero,.L36
        beq     a2,zero,.L36
        addi    sp,sp,-304
        sw      s0,296(sp)
        sw      s2,288(sp)
        sw      s3,284(sp)
        sw      ra,300(sp)
        sw      s1,292(sp)
        sw      s4,280(sp)
        sw      s5,276(sp)
        sw      s6,272(sp)
        sw      s7,268(sp)
        sw      s8,264(sp)
        sw      s9,260(sp)
        sw      s10,256(sp)
        mv      s0,a0
        mv      s3,a1
        mv      s2,a3
        bne     a3,zero,.L41
.L3:
        li      a5,21
        ble     s3,a5,.L2
        lbu     s1,4(s0)
        lbu     a4,5(s0)
        lbu     a3,7(s0)
        lbu     a5,6(s0)
        slli    a4,a4,16
        slli    s1,s1,24
        or      s1,s1,a4
        or      s1,s1,a3
        slli    a5,a5,8
        or      s1,s1,a5
        lbu     a4,0(s0)
        lbu     a6,1(s0)
        lbu     a0,2(s0)
        lbu     t1,3(s0)
        sw      s1,0(a2)
        lbu     a5,8(s0)
        lbu     a1,9(s0)
        lbu     a7,11(s0)
        lbu     a3,10(s0)
        slli    a1,a1,16
        slli    a5,a5,24
        or      a5,a5,a1
        slli    a3,a3,8
        or      a5,a5,a7
        or      a5,a5,a3
        sw      a5,4(a2)
        lbu     a3,12(s0)
        sb      a3,8(a2)
        lbu     a1,13(s0)
        sb      a1,9(a2)
        beq     s1,zero,.L2
        beq     a5,zero,.L2
        addi    a2,a3,-3
        andi    a2,a2,0xff
        li      a7,1
        bgtu    a2,a7,.L2
        bgtu    a1,a7,.L2
        slli    a4,a4,24
        slli    a6,a6,16
        or      a4,a4,a6
        or      a4,a4,t1
        slli    a0,a0,8
        li      a2,1903128576
        or      a4,a4,a0
        addi    a2,a2,-1690
        bne     a4,a2,.L2
        li      a4,399998976
        addi    a4,a4,1024
        divu    a4,a4,s1
        bleu    a4,a5,.L2
        bne     s2,zero,.L4
        mv      s2,a3
.L4:
        mul     a5,a5,s1
        li      a2,256
        li      a1,0
        mv      a0,sp
        addi    s3,s3,-8
        mul     s1,a5,s2
        call    q_memset
        ble     s1,zero,.L17
        li      t2,-65536
        li      t0,-16711680
        li      t6,16777216
        li      t1,14
        li      a6,0
        li      a1,0
        li      a4,0
        li      a7,0
        li      a0,0
        li      t3,255
        li      s4,254
        addi    t2,t2,255
        addi    t0,t0,-1
        addi    t6,t6,-1
        li      s5,255
        li      s6,64
        li      s7,128
        li      s8,192
        li      t5,499712
        li      t4,4
.L5:
        beq     a1,zero,.L7
        addi    a1,a1,-1
.L8:
        add     a5,t5,a4
        sb      a6,288(a5)
        sb      a7,289(a5)
        sb      a0,290(a5)
        beq     s2,t4,.L42
        add     a4,a4,s2
        bgt     s1,a4,.L5
.L17:
        li      a0,499712
        addi    a0,a0,288
        j       .L1
.L41:
        addi    a5,a3,-3
        li      a4,1
        bleu    a5,a4,.L3
.L2:
        li      a0,0
.L1:
        lw      ra,300(sp)
        lw      s0,296(sp)
        lw      s1,292(sp)
        lw      s2,288(sp)
        lw      s3,284(sp)
        lw      s4,280(sp)
        lw      s5,276(sp)
        lw      s6,272(sp)
        lw      s7,268(sp)
        lw      s8,264(sp)
        lw      s9,260(sp)
        lw      s10,256(sp)
        addi    sp,sp,304
        jr      ra
.L7:
        bge     t1,s3,.L8
        add     a2,s0,t1
        lbu     a5,0(a2)
        addi    a3,t1,1
        beq     a5,s4,.L43
        beq     a5,s5,.L44
        andi    a2,a5,192
        beq     a2,zero,.L45
        beq     a2,s6,.L46
        beq     a2,s7,.L47
        mv      t1,a3
        bne     a2,s8,.L10
        andi    a1,a5,63
        j       .L10
.L42:
        sb      t3,291(a5)
        addi    a4,a4,4
        blt     a4,s1,.L5
        j       .L17
.L36:
        li      a0,0
        ret
.L43:
        addi    a5,t1,3
        add     a3,s0,a3
        add     a5,s0,a5
        lbu     a6,0(a3)
        lbu     a7,2(a2)
        lbu     a0,0(a5)
        addi    t1,t1,4
.L10:
        slli    s10,a6,1
        slli    a3,a7,2
        slli    a5,t3,1
        add     a3,a3,a7
        add     s10,s10,a6
        slli    s9,a0,3
        add     a5,a5,t3
        add     s10,s10,a3
        sub     s9,s9,a0
        slli    a3,a7,8
        slli    a5,a5,2
        and     a2,a6,t2
        sub     a5,a5,t3
        or      a2,a2,a3
        add     s9,s10,s9
        add     s9,s9,a5
        and     a2,a2,t0
        slli    a3,a0,16
        or      a3,a2,a3
        andi    s9,s9,63
        slli    a2,s9,2
        slli    a5,t3,24
        and     a3,a3,t6
        add     a2,sp,a2
        or      a5,a3,a5
        sw      a5,0(a2)
        j       .L8
.L45:
        slli    a5,a5,2
        addi    a2,sp,256
        add     a5,a2,a5
        lbu     a6,-256(a5)
        lbu     a7,-255(a5)
        lbu     a0,-254(a5)
        lbu     t3,-253(a5)
        mv      t1,a3
        j       .L10
.L44:
        addi    a5,t1,4
        add     a3,s0,a3
        add     a5,s0,a5
        lbu     a6,0(a3)
        lbu     a7,2(a2)
        lbu     a0,3(a2)
        lbu     t3,0(a5)
        addi    t1,t1,5
        j       .L10
.L46:
        srai    s10,a5,4
        srai    a2,a5,2
        addi    t1,a7,-2
        addi    s9,a6,-2
        addi    a0,a0,-2
        andi    a6,s10,3
        andi    a7,a2,3
        andi    a5,a5,3
        add     a7,a7,t1
        add     a6,a6,s9
        add     a0,a5,a0
        andi    a6,a6,0xff
        andi    a7,a7,0xff
        andi    a0,a0,0xff
        mv      t1,a3
        j       .L10
.L47:
        add     a3,s0,a3
        lbu     a3,0(a3)
        andi    a5,a5,63
        addi    a5,a5,-32
        andi    a5,a5,0xff
        addi    a6,a6,-8
        addi    a0,a0,-8
        srai    a2,a3,4
        add     a6,a5,a6
        add     a0,a5,a0
        andi    a3,a3,15
        add     a6,a6,a2
        add     a7,a5,a7
        add     a0,a0,a3
        addi    t1,t1,2
        andi    a6,a6,0xff
        andi    a7,a7,0xff
        andi    a0,a0,0xff
        j       .L10

q_memset:
        beq     a2,zero,.L48
        add     a2,a0,a2
        sb      a1,0(a2)
.L48:
        ret

.end:
		nop

