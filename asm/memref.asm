loadw r0, [0xf00]
loadw ra, [123 + r1]
storew [r1 - 1], sp
storew [r5], sp
storew [r5 + 1], sp
storew [1 + r5], sp
storew [r5 + 5], sp
storew [r5 + 0x10], sp

