// memcpy for x86-64, replacing musl's src/string/x86_64/memcpy.s when
// build.rs builds musl (which then also uses its generic C memmove in place
// of src/string/x86_64/memmove.s).
//
// musl's assembly memcpy starts every copy with `rep movsq`, whose start-up
// cost is about 20 ns: a 7-byte copy took as long as a 256-byte one, and
// glibc's took a tenth of that. qiq programs mostly copy short strings, so
// copies below QIQ_MEMCPY_REP bytes here are done with unaligned 8-byte
// loads and stores, the first and last ones overlapping as needed, and only
// longer copies use `rep movsb`, which is the fastest way to copy them on
// x86-64 processors since 2013 (ERMS).
//
// musl compiles this with -ffreestanding, so the compiler doesn't turn the
// loops back into calls to memcpy.
#include <stdint.h>
#include <string.h>

#define QIQ_MEMCPY_REP 2048

typedef uint64_t __attribute__((__may_alias__, __aligned__(1))) qiq_u64;
typedef uint32_t __attribute__((__may_alias__, __aligned__(1))) qiq_u32;
typedef uint16_t __attribute__((__may_alias__, __aligned__(1))) qiq_u16;

void *memcpy(void *restrict dest, const void *restrict src, size_t n)
{
	unsigned char *d = dest;
	const unsigned char *s = src;

	if (n <= 16) {
		if (n >= 8) {
			uint64_t first = *(const qiq_u64 *)s, last = *(const qiq_u64 *)(s + n - 8);
			*(qiq_u64 *)d = first;
			*(qiq_u64 *)(d + n - 8) = last;
		} else if (n >= 4) {
			uint32_t first = *(const qiq_u32 *)s, last = *(const qiq_u32 *)(s + n - 4);
			*(qiq_u32 *)d = first;
			*(qiq_u32 *)(d + n - 4) = last;
		} else if (n >= 2) {
			uint16_t first = *(const qiq_u16 *)s, last = *(const qiq_u16 *)(s + n - 2);
			*(qiq_u16 *)d = first;
			*(qiq_u16 *)(d + n - 2) = last;
		} else if (n == 1) {
			*d = *s;
		}
		return dest;
	}

	if (n <= 32) {
		uint64_t a = *(const qiq_u64 *)s, b = *(const qiq_u64 *)(s + 8);
		uint64_t c = *(const qiq_u64 *)(s + n - 16), e = *(const qiq_u64 *)(s + n - 8);
		*(qiq_u64 *)d = a;
		*(qiq_u64 *)(d + 8) = b;
		*(qiq_u64 *)(d + n - 16) = c;
		*(qiq_u64 *)(d + n - 8) = e;
		return dest;
	}

	if (n < QIQ_MEMCPY_REP) {
		// 32 bytes at a time, then the last 32 bytes, which may overlap the
		// last chunk the loop copied.
		const unsigned char *tail = s + n - 32;
		uint64_t t0 = *(const qiq_u64 *)tail, t1 = *(const qiq_u64 *)(tail + 8);
		uint64_t t2 = *(const qiq_u64 *)(tail + 16), t3 = *(const qiq_u64 *)(tail + 24);
		unsigned char *end = d + n - 32;
		while (d < end) {
			uint64_t a = *(const qiq_u64 *)s, b = *(const qiq_u64 *)(s + 8);
			uint64_t c = *(const qiq_u64 *)(s + 16), e = *(const qiq_u64 *)(s + 24);
			*(qiq_u64 *)d = a;
			*(qiq_u64 *)(d + 8) = b;
			*(qiq_u64 *)(d + 16) = c;
			*(qiq_u64 *)(d + 24) = e;
			d += 32;
			s += 32;
		}
		*(qiq_u64 *)end = t0;
		*(qiq_u64 *)(end + 8) = t1;
		*(qiq_u64 *)(end + 16) = t2;
		*(qiq_u64 *)(end + 24) = t3;
		return dest;
	}

	__asm__ volatile ("rep movsb" : "+D"(d), "+S"(s), "+c"(n) : : "memory");
	return dest;
}
